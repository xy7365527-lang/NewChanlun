//! #1488 B：同判据的贡献核试验。整个模块仅 cfg(test)，不签发生产资格。
//! 当前 root 完整变化适配器独立枚举原件；正式 owner 接线前不得声称热路径已消全扫。
//! 失败消费并丢弃整份 pending 状态，不调用全量审计兜底。
use super::*;
use sc::capture_allocation::OwnedBudget;

type DigestSet = BusinessMap<NodeDigest, ()>;
struct ObjectEntry {
    original: RefValue,
    value: Object,
    group: Option<i64>,
}
struct WitnessEntry {
    original: RefValue,
    object_id: bj::TextValue,
    slot: i64,
}
struct Observation {
    count: usize,
    insufficient: bool,
    outside_domain: bool,
}
#[derive(Default, Debug)]
struct Work {
    objects: usize,
    shapes: usize,
    witnesses: usize,
    observations: usize,
    // 显式计入残余全扫；不把局部字段读取下降冒充完整增量。
    pending_objects: usize,
    partition_members: usize,
}
struct Contributions {
    root: Option<NodeDigest>,
    allocation: Option<OwnedBudget>,
    rule: bj::TextValue,
    raw: BusinessMap<i64, Source>,
    objects: BusinessMap<NodeDigest, ObjectEntry>,
    ids: BusinessMap<bj::TextValue, ()>,
    groups: BusinessMap<i64, Group>,
    shapes: BusinessMap<bj::TextValue, Shape>,
    witnesses: BusinessMap<NodeDigest, WitnessEntry>,
    seen: BusinessMap<(bj::TextValue, i64), ()>,
    raw_objects: BusinessMap<i64, DigestSet>,
    group_shapes: BusinessMap<i64, DigestSet>,
    shape_witnesses: BusinessMap<bj::TextValue, DigestSet>,
    observations: BusinessMap<NodeDigest, Observation>,
    insufficient: usize,
    outside_domain: usize,
    work: Work,
}
struct Pending(Contributions);
impl Pending {
    // 仅 fixture 的“下一代”操作；不是事务 COMMIT，也没有生产提升函数。
    fn next_fixture(self) -> Contributions {
        self.0
    }
    fn counts(&self) -> Counts {
        self.0.counts()
    }
}
struct RawEdit {
    coord: i64,
    old: Option<Source>,
    new: Option<Source>,
}
struct Members {
    removed: sc::CodecVec<NodeDigest>,
    added: sc::CodecVec<(usize, RefValue)>,
    order: sc::CodecVec<NodeDigest>,
    positions: BusinessMap<NodeDigest, usize>,
}
/// 私有测试资格；字段不对 generator 暴露，构造实际读取并核完整 root。
struct FixtureCut {
    before: Option<NodeDigest>,
    root: RefValue,
    raw_prefix: usize,
    raw: sc::CodecVec<RawEdit>,
    objects: Members,
    witnesses: Members,
    observations: Members,
}
fn put<K: Eq + std::hash::Hash, V>(
    s: &(impl CodecRead + ?Sized),
    map: &mut BusinessMap<K, V>,
    key: K,
    value: V,
) -> Result<Option<V>, String> {
    s.native_map_room(map).map_err(err)?;
    map.insert(key, value).map_err(err)
}
fn push<T>(
    s: &(impl CodecRead + ?Sized),
    list: &mut sc::CodecVec<T>,
    value: T,
) -> Result<(), String> {
    s.native_codec_room(list, 1).map_err(err)?;
    list.try_push(value).map_err(err)
}
fn depend_coord(
    s: &(impl CodecRead + ?Sized),
    index: &mut BusinessMap<i64, DigestSet>,
    key: i64,
    record: NodeDigest,
) -> Result<(), String> {
    if !index.contains_key(&key) {
        put(s, index, key, bj::new_map(source_allocation(s)?.as_ref()))?;
    }
    put(
        s,
        index.get_mut(&key).expect("created dependency"),
        record,
        (),
    )?;
    Ok(())
}
fn forget_coord(index: &mut BusinessMap<i64, DigestSet>, key: i64, record: NodeDigest) {
    if let Some(set) = index.get_mut(&key) {
        set.remove(&record);
        if set.is_empty() {
            index.remove(&key);
        }
    }
}
fn extend_set(
    s: &(impl CodecRead + ?Sized),
    target: &mut DigestSet,
    source: Option<&DigestSet>,
) -> Result<(), String> {
    if let Some(source) = source {
        for (key, ()) in source.iter() {
            put(s, target, *key, ())?;
        }
    }
    Ok(())
}
impl Contributions {
    fn new(s: &(impl CodecRead + ?Sized), rule: &str) -> Result<Self, String> {
        let allocation = source_allocation(s)?;
        Ok(Self {
            root: None,
            rule: s.native_text(rule)?,
            raw: bj::new_map(allocation.as_ref()),
            objects: bj::new_map(allocation.as_ref()),
            ids: bj::new_map(allocation.as_ref()),
            groups: bj::new_map(allocation.as_ref()),
            shapes: bj::new_map(allocation.as_ref()),
            witnesses: bj::new_map(allocation.as_ref()),
            seen: bj::new_map(allocation.as_ref()),
            raw_objects: bj::new_map(allocation.as_ref()),
            group_shapes: bj::new_map(allocation.as_ref()),
            shape_witnesses: bj::new_map(allocation.as_ref()),
            observations: bj::new_map(allocation.as_ref()),
            insufficient: 0,
            outside_domain: 0,
            work: Work::default(),
            allocation,
        })
    }
    fn same_ledger(&self, s: &(impl CodecRead + ?Sized)) -> Result<(), String> {
        match (&self.allocation, source_allocation(s)?) {
            (None, None) => Ok(()),
            (Some(a), Some(b)) if a.same_ledger(&b) => Ok(()),
            _ => Err(err("来源贡献跨工作账本")),
        }
    }
    fn counts(&self) -> Counts {
        Counts {
            objects: self.objects.len(),
            groups: self.groups.len(),
            windows: self.groups.len().saturating_sub(2),
            effective: self.raw.len(),
            insufficient: self.insufficient,
            outside_domain: self.outside_domain,
        }
    }
    fn invalidate_witnesses(
        &self,
        s: &(impl CodecRead + ?Sized),
        id: &str,
        dirty: &mut DigestSet,
    ) -> Result<(), String> {
        extend_set(s, dirty, self.shape_witnesses.get(id))
    }
    fn remove_object(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        key: NodeDigest,
        groups: &mut BusinessMap<i64, ()>,
        witnesses: &mut DigestSet,
    ) -> Result<(), String> {
        let old = self
            .objects
            .remove(&key)
            .ok_or_else(|| err("删除不存在的来源对象"))?;
        self.ids.remove(old.value.id.as_str());
        for coord in old.value.sources.iter() {
            forget_coord(&mut self.raw_objects, *coord, key);
        }
        if let Some(index) = old.group {
            self.groups.remove(&index);
            put(s, groups, index, ())?;
        }
        if let Some(shape) = self.shapes.remove(old.value.id.as_str()) {
            self.invalidate_witnesses(s, old.value.id.as_str(), witnesses)?;
            for index in shape.groups {
                forget_coord(&mut self.group_shapes, index, key);
            }
        }
        Ok(())
    }
    fn remove_witness(&mut self, key: NodeDigest) -> Result<(), String> {
        let old = self
            .witnesses
            .remove(&key)
            .ok_or_else(|| err("删除不存在的来源见证"))?;
        if let Some(set) = self.shape_witnesses.get_mut(old.object_id.as_str()) {
            set.remove(&key);
            if set.is_empty() {
                self.shape_witnesses.remove(old.object_id.as_str());
            }
        }
        self.seen.remove(&(old.object_id, old.slot));
        Ok(())
    }
    fn advance(
        mut self,
        s: &mut (impl SourceFieldStore + ?Sized),
        cut: FixtureCut,
    ) -> Result<Pending, String> {
        self.same_ledger(s)?;
        if cut.before != self.root {
            return Err(err("来源贡献前根不匹配"));
        }
        // fixture 的完整当前原件复核。真实 owner 应以闭包资格替换，而非删除此门。
        s.verify_ref(cut.root.view(), sc::Mode::Public)
            .map_err(err)?;
        self.work = Work::default();
        let allocation = source_allocation(s)?;
        let mut dirty_objects = bj::new_map(allocation.as_ref());
        let mut dirty_groups = bj::new_map(allocation.as_ref());
        let mut dirty_shapes = bj::new_map(allocation.as_ref());
        let mut dirty_witnesses = bj::new_map(allocation.as_ref());
        for edit in &cut.raw {
            if self.raw.get(&edit.coord).copied() != edit.old {
                return Err(err("来源贡献原账旧赢家不匹配"));
            }
            extend_set(s, &mut dirty_objects, self.raw_objects.get(&edit.coord))?;
        }
        // 旧赢家全部撤销后再加入；两个身份交换坐标不会被中间态误判。
        for edit in &cut.raw {
            self.raw.remove(&edit.coord);
        }
        for edit in &cut.raw {
            if let Some(value) = edit.new {
                put(s, &mut self.raw, edit.coord, value)?;
            }
        }
        let mut explicit_removed = bj::new_map(allocation.as_ref());
        for key in &cut.objects.removed {
            put(s, &mut explicit_removed, *key, ())?;
            put(s, &mut dirty_objects, *key, ())?;
        }
        let mut recheck = bj::new_work(allocation.as_ref());
        for (key, ()) in dirty_objects.iter() {
            if !explicit_removed.contains_key(key) {
                let old = self
                    .objects
                    .get(key)
                    .ok_or_else(|| err("来源依赖索引悬空"))?;
                let ordinal = *cut
                    .objects
                    .positions
                    .get(key)
                    .ok_or_else(|| err("当前根漏报来源对象删除"))?;
                push(
                    s,
                    &mut recheck,
                    (ordinal, s.native_ref_clone(&old.original).map_err(err)?),
                )?;
            }
        }
        for (key, ()) in dirty_objects.iter() {
            self.remove_object(s, *key, &mut dirty_groups, &mut dirty_witnesses)?;
        }
        for (ordinal, record) in cut.objects.added {
            push(s, &mut recheck, (ordinal, record))?;
        }
        recheck.sort_unstable_by_key(|(ordinal, _)| *ordinal);
        s.native_admit(
            tb02a::PROFILE
                .len()
                .checked_add(self.rule.as_str().len())
                .and_then(|n| n.checked_add(2 * std::mem::size_of::<[usize; 2]>()))
                .ok_or_else(|| err(s.native_overflow()))?,
        )
        .map_err(err)?;
        let producer = Cc004Producer::new(tb02a::PROFILE, self.rule.as_str());
        let mut checked = bj::new_map(allocation.as_ref());
        for (_, record) in recheck {
            let key = digest(&record)?;
            let value = audit_object_record(
                s,
                &record,
                &self.raw,
                &producer,
                &mut self.ids,
                &mut self.groups,
                &mut checked,
                None,
                cut.raw_prefix,
                false,
                None,
                None,
            )?;
            self.work.objects += 1;
            let group = if value.kind == CC005_GROUP_KIND {
                let p = value.fields.get_admitted(s, "payload_json")?;
                let p = s.lookup_ref(p.view(), &[PathStep::JsonText]).map_err(err)?;
                let index = decimal(&text(s, &p, "group_index")?)?;
                put(s, &mut dirty_groups, index, ())?;
                Some(index)
            } else {
                None
            };
            for coord in value.sources.iter() {
                depend_coord(s, &mut self.raw_objects, *coord, key)?;
            }
            if value.kind == CC006_SHAPE_KIND {
                put(s, &mut dirty_shapes, key, ())?;
            }
            if put(
                s,
                &mut self.objects,
                key,
                ObjectEntry {
                    original: record,
                    value,
                    group,
                },
            )?
            .is_some()
            {
                return Err(err("对象身份重复或修订不符"));
            }
        }
        // 尚待独立 prefix/request 证据接线的全体性门保持原样；绝不只审变化步骤。
        let pending = source_pending::audit(
            s,
            cut.objects
                .order
                .iter()
                .map(|key| &self.objects.get(key).expect("fixture成员完整性已核").value),
            &self.raw,
            &self.groups,
            None,
        )?;
        self.work.pending_objects = cut.objects.order.len();
        let mut partition = bj::new_map(allocation.as_ref());
        for (expected, (index, group)) in
            sorted_numeric_entries(s, &self.groups)?.iter().enumerate()
        {
            if *index != expected as i64 {
                return Err(err("组合并序号不连续"));
            }
            for coord in &group.members {
                self.work.partition_members += 1;
                if put(s, &mut partition, *coord, ())?.is_some() {
                    return Err(err("同cut组成员重叠"));
                }
            }
        }
        for coord in pending {
            if put(s, &mut partition, coord, ())?.is_some() {
                return Err(err("同cut组成员与初始未定来源重叠"));
            }
        }
        if partition.len() != self.raw.len()
            || partition.iter().any(|(c, _)| !self.raw.contains_key(c))
        {
            return Err(err("同cut组未完整覆盖生效原始账"));
        }
        for (index, ()) in dirty_groups.iter() {
            extend_set(s, &mut dirty_shapes, self.group_shapes.get(index))?;
        }
        // shape集合先核所有受影响窗口，再处理三槽见证。
        let mut shape_order = bj::new_work(allocation.as_ref());
        for (key, ()) in dirty_shapes.iter() {
            let ordinal = *cut
                .objects
                .positions
                .get(key)
                .ok_or_else(|| err("形态依赖索引悬空"))?;
            push(s, &mut shape_order, (ordinal, *key))?;
        }
        shape_order.sort_unstable_by_key(|(ordinal, _)| *ordinal);
        for (_, key) in shape_order {
            let object = &self
                .objects
                .get(&key)
                .ok_or_else(|| err("缺当前形态原件"))?
                .value;
            self.invalidate_witnesses(s, object.id.as_str(), &mut dirty_witnesses)?;
            if let Some(old) = self.shapes.remove(object.id.as_str()) {
                for index in old.groups {
                    forget_coord(&mut self.group_shapes, index, key);
                }
            }
            let shape = audit_shape_record(
                s,
                object,
                &self.raw,
                &self.groups,
                &producer,
                &mut checked,
                None,
                cut.raw_prefix,
                None,
                None,
            )?;
            self.work.shapes += 1;
            for index in shape.groups {
                depend_coord(s, &mut self.group_shapes, index, key)?;
            }
            put(
                s,
                &mut self.shapes,
                s.native_text(object.id.as_str())?,
                shape,
            )?;
        }
        let mut removed_witnesses = bj::new_map(allocation.as_ref());
        for key in &cut.witnesses.removed {
            put(s, &mut removed_witnesses, *key, ())?;
            put(s, &mut dirty_witnesses, *key, ())?;
        }
        let mut recheck = bj::new_work(allocation.as_ref());
        for (key, ()) in dirty_witnesses.iter() {
            if !removed_witnesses.contains_key(key) {
                let old = self
                    .witnesses
                    .get(key)
                    .ok_or_else(|| err("见证依赖索引悬空"))?;
                let ordinal = *cut
                    .witnesses
                    .positions
                    .get(key)
                    .ok_or_else(|| err("当前根漏报见证删除"))?;
                push(
                    s,
                    &mut recheck,
                    (ordinal, s.native_ref_clone(&old.original).map_err(err)?),
                )?;
            }
        }
        for (key, ()) in dirty_witnesses.iter() {
            self.remove_witness(*key)?;
        }
        for (ordinal, record) in cut.witnesses.added {
            push(s, &mut recheck, (ordinal, record))?;
        }
        recheck.sort_unstable_by_key(|(ordinal, _)| *ordinal);
        for (_, record) in recheck {
            let key = digest(&record)?;
            let (object_id, slot) = audit_witness_record(
                s,
                &record,
                &self.raw,
                &self.groups,
                &self.shapes,
                &mut self.seen,
                None,
                None,
            )?;
            self.work.witnesses += 1;
            if !self.shape_witnesses.contains_key(object_id.as_str()) {
                put(
                    s,
                    &mut self.shape_witnesses,
                    s.native_text(object_id.as_str())?,
                    bj::new_map(allocation.as_ref()),
                )?;
            }
            put(
                s,
                self.shape_witnesses
                    .get_mut(object_id.as_str())
                    .expect("created witness index"),
                key,
                (),
            )?;
            put(
                s,
                &mut self.witnesses,
                key,
                WitnessEntry {
                    original: record,
                    object_id,
                    slot,
                },
            )?;
        }
        // 每条seen只指向本cut形态、slot属于0..3、(id,slot)唯一，因此总数相等蕴含每个形态恰三槽。
        if self.seen.len() != self.shapes.len() * 3 {
            return Err(err("形态缺完整三槽见证"));
        }
        for key in cut.observations.removed {
            let old = self
                .observations
                .get_mut(&key)
                .ok_or_else(|| err("删除不存在的来源观察"))?;
            old.count -= 1;
            self.insufficient -= usize::from(old.insufficient);
            self.outside_domain -= usize::from(old.outside_domain);
            if old.count == 0 {
                self.observations.remove(&key);
            }
        }
        for (_, record) in cut.observations.added {
            let key = digest(&record)?;
            let kind = text(s, &record, "kind")?;
            let insufficient = kind == "insufficient_knowledge";
            let outside_domain = kind == "domain_not_satisfied";
            self.insufficient += usize::from(insufficient);
            self.outside_domain += usize::from(outside_domain);
            self.work.observations += 1;
            if let Some(old) = self.observations.get_mut(&key) {
                old.count += 1;
            } else {
                put(
                    s,
                    &mut self.observations,
                    key,
                    Observation {
                        count: 1,
                        insufficient,
                        outside_domain,
                    },
                )?;
            }
        }
        self.root = Some(digest(&cut.root)?);
        Ok(Pending(self))
    }
}

impl Members {
    /// 仅测试适配：完整当前 root 原件与旧贡献的多重集差，不接收生成器的变化/计数。
    fn fixture(
        s: &mut (impl SourceFieldStore + ?Sized),
        root: &RefValue,
        name: &str,
        previous: impl Iterator<Item = (NodeDigest, usize)>,
    ) -> Result<Self, String> {
        let allocation = source_allocation(s)?;
        let mut remaining = bj::new_map(allocation.as_ref());
        for (key, count) in previous {
            put(s, &mut remaining, key, count)?;
        }
        let array = field(s, root, name)?;
        let mut removed = bj::new_work(allocation.as_ref());
        let mut added = bj::new_work(allocation.as_ref());
        let mut order = bj::new_work(allocation.as_ref());
        let mut positions = bj::new_map(allocation.as_ref());
        for (ordinal, record) in elements(s, &array)?.into_iter().enumerate() {
            let key = digest(&record)?;
            push(s, &mut order, key)?;
            put(s, &mut positions, key, ordinal)?;
            match remaining.get_mut(&key) {
                Some(count) if *count > 0 => *count -= 1,
                _ => push(s, &mut added, (ordinal, record))?,
            }
        }
        for (key, count) in remaining.iter() {
            for _ in 0..*count {
                push(s, &mut removed, *key)?;
            }
        }
        Ok(Self {
            removed,
            added,
            order,
            positions,
        })
    }
}
impl FixtureCut {
    fn bind(
        s: &mut (impl SourceFieldStore + ?Sized),
        root: sc::RefView<'_>,
        raw: RawEventsView<'_>,
        previous: &Contributions,
    ) -> Result<Self, String> {
        previous.same_ledger(s)?;
        match (&previous.allocation, raw.budget()) {
            (None, None) => {}
            (Some(a), Some(b)) if a.same_ledger(b) => {}
            _ => return Err(err("来源原账与贡献不属同账")),
        }
        s.verify_ref(root, sc::Mode::Public).map_err(err)?;
        let allocation = source_allocation(s)?;
        let mut current = bj::new_map(allocation.as_ref());
        for event in bj::effective(raw, None)?.iter() {
            let coord = bj::coord(event)?;
            let wire = bj::RawReference::new(event, false)?;
            let numeric = bj::RawReference::new(event, true)?;
            let source = Source {
                wire: super::super::history::hash_json_in_store(s, wire.view(), None, None)?,
                numeric: super::super::history::hash_json_in_store(s, numeric.view(), None, None)?,
                raw: super::super::history::hash_json_in_store(s, event, None, None)?,
                bar: source_pending::raw_bar(s, event, None, None)?,
            };
            put(s, &mut current, coord, source)?;
        }
        let mut edits = bj::new_work(allocation.as_ref());
        for (coord, old) in previous.raw.iter() {
            let new = current.get(coord).copied();
            if new != Some(*old) {
                push(
                    s,
                    &mut edits,
                    RawEdit {
                        coord: *coord,
                        old: Some(*old),
                        new,
                    },
                )?;
            }
        }
        for (coord, new) in current.iter() {
            if !previous.raw.contains_key(coord) {
                push(
                    s,
                    &mut edits,
                    RawEdit {
                        coord: *coord,
                        old: None,
                        new: Some(*new),
                    },
                )?;
            }
        }
        let root = s.native_built_ref(&built(root)?).map_err(err)?;
        let objects = Members::fixture(
            s,
            &root,
            "objects",
            previous.objects.iter().map(|(k, _)| (*k, 1)),
        )?;
        let witnesses = Members::fixture(
            s,
            &root,
            "witnesses",
            previous.witnesses.iter().map(|(k, _)| (*k, 1)),
        )?;
        let observations = Members::fixture(
            s,
            &root,
            "observations",
            previous.observations.iter().map(|(k, v)| (*k, v.count)),
        )?;
        Ok(Self {
            before: previous.root,
            root,
            raw_prefix: raw.len(),
            raw: edits,
            objects,
            witnesses,
            observations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn full(
        s: &mut sc::MemoryNodes,
        root: &ContentRef,
        raw: &[Value],
        rule: &str,
    ) -> Result<Counts, String> {
        frozen_oracle::audit_generated_in_store(
            &mut super::super::super::audit_store::StoreView::for_audit(s),
            root.into(),
            raw.into(),
            tb02a::PROFILE,
            rule,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }
    #[test]
    fn nonempty_root_and_unchanged_cut_match_independent_oracle() {
        let (mut s, root, raw, rule) = super::super::tests::fixture();
        let expected = full(&mut s, &root, &raw, &rule).unwrap();
        assert!(expected.groups > 2 && expected.objects > 0, "必须非空");
        let initial = Contributions::new(&s, &rule).unwrap();
        let cut = FixtureCut::bind(&mut s, (&root).into(), (&raw).into(), &initial).unwrap();
        let pending = initial.advance(&mut s, cut).unwrap();
        assert_eq!(pending.counts(), expected);
        let current = pending.next_fixture();
        let cut = FixtureCut::bind(&mut s, (&root).into(), (&raw).into(), &current).unwrap();
        let pending = current.advance(&mut s, cut).unwrap();
        assert_eq!(pending.counts(), expected);
        assert_eq!(
            (
                pending.0.work.objects,
                pending.0.work.shapes,
                pending.0.work.witnesses,
                pending.0.work.observations
            ),
            (0, 0, 0, 0)
        );
        assert!(pending.0.work.pending_objects > 0, "残余全体性扫描照实记录");
    }
    fn start(
        s: &mut sc::MemoryNodes,
        root: &ContentRef,
        raw: &[Value],
        rule: &str,
    ) -> Contributions {
        let state = Contributions::new(s, rule).unwrap();
        let cut = FixtureCut::bind(s, root.into(), raw.into(), &state).unwrap();
        state.advance(s, cut).unwrap().next_fixture()
    }
    fn collection(
        s: &mut sc::MemoryNodes,
        root: &ContentRef,
        name: &str,
        items: Vec<NodeDigest>,
    ) -> ContentRef {
        super::super::tests::replace_field(
            s,
            root,
            super::super::tests::STATE,
            name,
            V::Array(items.into_iter().map(V::Edge).collect()),
        )
    }
    fn members(s: &mut sc::MemoryNodes, root: &ContentRef, name: &str) -> Vec<NodeDigest> {
        let array = field(s, root, name).unwrap();
        elements(s, &array)
            .unwrap()
            .iter()
            .map(|r| digest(r).unwrap())
            .collect()
    }
    #[test]
    fn four_cuts_add_replace_remove_observations_keep_multiset_counts() {
        let (mut s, root, raw, rule) = super::super::tests::fixture();
        let mut state = start(&mut s, &root, &raw, &rule);
        let base = members(&mut s, &root, "observations");
        let insuff = sc::write_value(
            &mut s,
            &V::Object(vec![(
                "kind".into(),
                V::String("insufficient_knowledge".into()),
            )]),
        )
        .unwrap()
        .digest;
        let outside = sc::write_value(
            &mut s,
            &V::Object(vec![(
                "kind".into(),
                V::String("domain_not_satisfied".into()),
            )]),
        )
        .unwrap()
        .digest;
        for extras in [vec![insuff, insuff], vec![outside], vec![]] {
            let mut items = base.clone();
            items.extend(extras);
            let next = collection(&mut s, &root, "observations", items);
            let expected = full(&mut s, &next, &raw, &rule).unwrap();
            let cut = FixtureCut::bind(&mut s, (&next).into(), (&raw).into(), &state).unwrap();
            let pending = state.advance(&mut s, cut).unwrap();
            assert_eq!(pending.counts(), expected);
            assert_eq!(
                (
                    pending.0.work.objects,
                    pending.0.work.shapes,
                    pending.0.work.witnesses
                ),
                (0, 0, 0)
            );
            assert_eq!(
                pending.counts().windows,
                pending.counts().groups.saturating_sub(2)
            );
            state = pending.next_fixture();
        }
    }
    #[test]
    fn missing_and_duplicate_witness_slots_match_frozen_rejection() {
        let (mut s, root, raw, rule) = super::super::tests::fixture();
        let original = members(&mut s, &root, "witnesses");
        assert!(!original.is_empty());
        for duplicate in [false, true] {
            let state = start(&mut s, &root, &raw, &rule);
            let mut items = original.clone();
            let reason = if duplicate {
                items.push(items[0]);
                "见证槽位重复/越界"
            } else {
                items.remove(0);
                "形态缺完整三槽见证"
            };
            let next = collection(&mut s, &root, "witnesses", items);
            let expected = full(&mut s, &next, &raw, &rule).unwrap_err();
            assert!(expected.contains(reason));
            let cut = FixtureCut::bind(&mut s, (&next).into(), (&raw).into(), &state).unwrap();
            let actual = state
                .advance(&mut s, cut)
                .err()
                .expect("不得容忍缺槽或重槽");
            assert_eq!(actual, expected);
        }
    }
    #[test]
    fn duplicate_object_identity_matches_frozen_rejection() {
        let (mut s, root, raw, rule) = super::super::tests::fixture();
        let state = start(&mut s, &root, &raw, &rule);
        let mut items = members(&mut s, &root, "objects");
        items.push(items[0]);
        let next = collection(&mut s, &root, "objects", items);
        let expected = full(&mut s, &next, &raw, &rule).unwrap_err();
        let cut = FixtureCut::bind(&mut s, (&next).into(), (&raw).into(), &state).unwrap();
        assert_eq!(state.advance(&mut s, cut).err().unwrap(), expected);
    }
    #[test]
    fn source_revision_with_unchanged_prices_invalidates_old_facts() {
        let (mut s, root, mut raw, rule) = super::super::tests::fixture();
        let state = start(&mut s, &root, &raw, &rule);
        let mut revision = raw[0].clone();
        let rev = revision["revision"].as_i64().unwrap();
        revision["revision"] = json!(rev + 1);
        revision["seq"] = json!(raw.len());
        revision["event_id"] = json!("source-delta-revision-with-identical-ohlc");
        raw.push(revision);
        let expected = full(&mut s, &root, &raw, &rule).unwrap_err();
        let cut = FixtureCut::bind(&mut s, (&root).into(), (&raw).into(), &state).unwrap();
        assert!(!cut.raw.is_empty(), "相同OHLC不能隐藏来源修订");
        let actual = state
            .advance(&mut s, cut)
            .err()
            .expect("旧事实不得借新原账放行");
        assert_eq!(actual, expected);
    }
    #[test]
    fn mismatched_previous_root_does_not_produce_pending_state() {
        let (mut s, root, raw, rule) = super::super::tests::fixture();
        let state = start(&mut s, &root, &raw, &rule);
        let mut cut = FixtureCut::bind(&mut s, (&root).into(), (&raw).into(), &state).unwrap();
        cut.before = None;
        assert!(state
            .advance(&mut s, cut)
            .err()
            .unwrap()
            .contains("前根不匹配"));
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_contribution_containers_charge_same_ledger_and_reject_foreign_reader() {
        struct BudgetRead(OwnedBudget);
        impl NodeSource for BudgetRead {
            fn read(&mut self, _: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
                Err(sc::CodecError::new(sc::code::SOURCE, "此探针不读取原件"))
            }
            fn allocation_budget(&self) -> Option<OwnedBudget> {
                Some(self.0.clone())
            }
        }
        impl CodecRead for BudgetRead {
            fn generic_source(&mut self) -> &mut dyn NodeSource {
                self
            }
        }
        let a = BudgetRead(OwnedBudget::new(1024 * 1024));
        let b = BudgetRead(OwnedBudget::new(1024 * 1024));
        let before = a.0.snapshot().live_requested_layout_bytes;
        let mut state = Contributions::new(&a, "test-rule").unwrap();
        state.same_ledger(&a).unwrap();
        assert!(state.same_ledger(&b).unwrap_err().contains("跨工作账本"));
        assert!(state.same_ledger(&sc::MemoryNodes::default()).is_err());
        let source = Source {
            wire: [1; 32],
            numeric: [2; 32],
            raw: [3; 32],
            bar: [4; 32],
        };
        put(&a, &mut state.raw, 7, source).unwrap();
        depend_coord(&a, &mut state.raw_objects, 7, [5; 32]).unwrap();
        assert_eq!(state.counts().effective, 1);
        assert!(a.0.snapshot().live_requested_layout_bytes > before);
        assert_eq!(b.0.snapshot().live_requested_layout_bytes, 0);
        drop(state);
        assert_eq!(a.0.snapshot().live_requested_layout_bytes, before);
    }
}
