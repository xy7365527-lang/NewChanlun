//! #1488 C2：owner 私有完整有序材料差异。当前接线仅在旧来源审计成功后调用。
//! 这不是 B2 增量业务审计；完整来源/投影判据仍由现有审计器执行。
//! 两端均有同域 Public 完整闭包资格时才跳过共享 M 子树，绝不接生成器 changes。
use super::super::audit_retention::vec_bytes;
use super::super::audit_store::CodecRead;
use super::{construction_state::stale, native, RawAdvanceBinding, SnapshotSeal};
use newchan_rust::s_content::{self as sc, path::SequenceFragment, NodeDigest, SequenceRef};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Declaration {
    pub(super) digest: NodeDigest,
    kind: sc::Kind,
    extent: Option<u64>,
}
impl Declaration {
    pub(super) fn read(reference: sc::RefView<'_>) -> Result<Self, String> {
        Ok(Self {
            digest: reference.digest().map_err(|e| stale(&e.to_string()))?,
            kind: reference.kind(),
            extent: reference.extent(),
        })
    }
    fn reference(
        &self,
        s: &(impl CodecRead + ?Sized),
    ) -> Result<super::super::audit_store::RefValue, String> {
        s.native_built_ref(&sc::Built {
            digest: self.digest,
            kind: self.kind,
            extent: self.extent,
        })
        .map_err(|e| stale(&e.to_string()))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Span {
    pub(super) ordinal: u64,
    pub(super) sequence: SequenceRef,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Edit {
    Retain { old: Span, current: Span },
    Remove { ordinal: u64, value: NodeDigest },
    Add { ordinal: u64, value: NodeDigest },
}
pub(super) struct SequenceDelta {
    pub(super) old: Option<SequenceRef>,
    pub(super) current: Option<SequenceRef>,
    pub(super) edits: Vec<Edit>,
}
impl SequenceDelta {
    fn retained_bytes(&self) -> usize {
        vec_bytes::<Edit>(self.edits.capacity())
    }
}
fn grow<T>(b: &native::Budget, v: &mut Vec<T>, value: T) -> Result<(), String> {
    b.vec_grow(v, 1).map_err(|_| stale("集合差异额度不足"))?;
    v.push(value);
    Ok(())
}
fn split(
    s: &mut (impl CodecRead + ?Sized),
    b: &native::Budget,
    stack: &mut Vec<Span>,
) -> Result<(), String> {
    let span = stack.pop().ok_or_else(|| stale("空序列前沿"))?;
    match sc::path::sequence_fragment(s.generic_source(), span.sequence)
        .map_err(|e| stale(&e.to_string()))?
    {
        SequenceFragment::Branch { left, right } => {
            grow(
                b,
                stack,
                Span {
                    ordinal: span.ordinal + left.count(),
                    sequence: right,
                },
            )?;
            grow(
                b,
                stack,
                Span {
                    ordinal: span.ordinal,
                    sequence: left,
                },
            )?;
            Ok(())
        }
        _ => Err(stale("非分支序列不能拆分")),
    }
}
fn leaf(s: &mut (impl CodecRead + ?Sized), sequence: SequenceRef) -> Result<NodeDigest, String> {
    match sc::path::sequence_fragment(s.generic_source(), sequence)
        .map_err(|e| stale(&e.to_string()))?
    {
        SequenceFragment::Leaf(value) => Ok(value),
        _ => Err(stale("集合成员不是规范单叶")),
    }
}
/// 私有：调用方先证明两端完整材料域；公共 SequenceRef 自身从不授资格。
fn difference(
    s: &mut (impl CodecRead + ?Sized),
    b: &native::Budget,
    old: Option<SequenceRef>,
    current: Option<SequenceRef>,
) -> Result<SequenceDelta, String> {
    let mut before = Vec::new();
    let mut after = Vec::new();
    for (sequence, stack) in [(old, &mut before), (current, &mut after)] {
        if let Some(sequence) = sequence {
            if sequence.is_empty() {
                // 即使空序列也触碰原件，不能只认 empty 摘要。
                sc::path::sequence_fragment(s.generic_source(), sequence)
                    .map_err(|e| stale(&e.to_string()))?;
            } else {
                grow(
                    b,
                    stack,
                    Span {
                        ordinal: 0,
                        sequence,
                    },
                )?;
            }
        }
    }
    let mut edits = Vec::new();
    while let (Some(a), Some(z)) = (before.last().copied(), after.last().copied()) {
        if a.sequence == z.sequence {
            before.pop();
            after.pop();
            grow(b, &mut edits, Edit::Retain { old: a, current: z })?;
        } else if a.sequence.count() == 1 && z.sequence.count() == 1 {
            before.pop();
            after.pop();
            grow(
                b,
                &mut edits,
                Edit::Remove {
                    ordinal: a.ordinal,
                    value: leaf(s, a.sequence)?,
                },
            )?;
            grow(
                b,
                &mut edits,
                Edit::Add {
                    ordinal: z.ordinal,
                    value: leaf(s, z.sequence)?,
                },
            )?;
        } else if a.sequence.count() >= z.sequence.count() && a.sequence.count() > 1 {
            split(s, b, &mut before)?;
        } else {
            split(s, b, &mut after)?;
        }
    }
    for (removed, stack) in [(true, &mut before), (false, &mut after)] {
        while let Some(span) = stack.last().copied() {
            if span.sequence.count() > 1 {
                split(s, b, stack)?;
            } else {
                stack.pop();
                let value = leaf(s, span.sequence)?;
                grow(
                    b,
                    &mut edits,
                    if removed {
                        Edit::Remove {
                            ordinal: span.ordinal,
                            value,
                        }
                    } else {
                        Edit::Add {
                            ordinal: span.ordinal,
                            value,
                        }
                    },
                )?;
            }
        }
    }
    let released = vec_bytes::<Span>(before.capacity()) + vec_bytes::<Span>(after.capacity());
    drop((before, after));
    b.release(released);
    Ok(SequenceDelta {
        old,
        current,
        edits,
    })
}
fn array(
    s: &mut (impl CodecRead + ?Sized),
    r: sc::RefView<'_>,
    key: &str,
) -> Result<(Declaration, SequenceRef), String> {
    let value = s
        .lookup_field_ref(r, key)
        .map_err(|e| stale(&e.to_string()))?;
    let sequence = sc::path::array_sequence_refview(s.generic_source(), value.view())
        .map_err(|e| stale(&e.to_string()))?;
    Ok((Declaration::read(value.view())?, sequence))
}
#[derive(Clone, Copy)]
struct Knowledge {
    ordinal: u64,
    requests: Declaration,
}
/// 跨事务只保存已提交根与已核 CC054 定位，不保存调用者 changes 或业务成功布尔量。
pub(super) struct CommittedSet {
    pub(super) root: Declaration,
    raw: RawAdvanceBinding,
    knowledge: Option<Knowledge>,
}
pub(super) struct Seed {
    pub(super) raw: RawAdvanceBinding,
    pub(super) seal: SnapshotSeal,
    pub(super) previous: Option<Declaration>,
    knowledge: Option<Knowledge>,
    bootstrap: bool,
}
impl Seed {
    pub(super) fn new(
        raw: RawAdvanceBinding,
        seal: SnapshotSeal,
        previous: Option<Declaration>,
        committed: Option<CommittedSet>,
    ) -> Result<Self, String> {
        if raw.owner_life() != seal.life {
            return Err(stale("集合raw与快照不属同owner"));
        }
        let (knowledge, bootstrap) = match committed {
            Some(saved) => {
                if Some(saved.root) != previous || saved.raw.domain() != raw.domain() {
                    return Err(stale("集合旧根/raw域与已提交前身不同"));
                }
                (saved.knowledge, false)
            }
            None => (None, true),
        };
        Ok(Self {
            raw,
            seal,
            previous,
            knowledge,
            bootstrap,
        })
    }
}
/// 非 Clone；材料事实和 effects 闭合分开。构造只可从 owner 安装的 Seed 进入。
pub(super) struct PendingSet {
    pub(super) previous: Option<Declaration>,
    pub(super) current: Declaration,
    pub(super) raw: RawAdvanceBinding,
    pub(super) seal: SnapshotSeal,
    pub(super) material_epoch: u64,
    pub(super) arrays: [SequenceDelta; 3],
    pub(super) requests: Option<SequenceDelta>,
    knowledge: Option<Knowledge>,
    stages: Option<StageSeal>,
    closed: bool,
}
struct StageSeal {
    material_epoch: u64,
    ordered_events: usize,
    stage_count: usize,
    total_changes: u64,
}
impl PendingSet {
    pub(super) fn retained_bytes(&self) -> usize {
        self.arrays
            .iter()
            .map(SequenceDelta::retained_bytes)
            .sum::<usize>()
            + self
                .requests
                .as_ref()
                .map_or(0, SequenceDelta::retained_bytes)
    }
    pub(super) fn close_stages(
        &mut self,
        material_epoch: u64,
        ordered_events: usize,
        stage_count: usize,
        total_changes: u64,
    ) -> Result<(), String> {
        if self.stages.is_some()
            || material_epoch < self.material_epoch
            || stage_count > ordered_events
        {
            return Err(stale("集合stage封口重复或事件/材料边界不符"));
        }
        self.stages = Some(StageSeal {
            material_epoch,
            ordered_events,
            stage_count,
            total_changes,
        });
        Ok(())
    }
    pub(super) fn close_against_persisted(
        &mut self,
        current: sc::RefView<'_>,
        raw: RawAdvanceBinding,
    ) -> Result<(), String> {
        if self.closed
            || self.stages.is_none()
            || Declaration::read(current)? != self.current
            || self.raw != raw
        {
            return Err(stale("实际持久投影根/raw与材料cut不一致"));
        }
        // 调用位置在 append_owner_cut 完成全部身份/辅助投影双向核验之后。
        self.closed = true;
        Ok(())
    }
    pub(super) fn check_commit(
        &self,
        seal: SnapshotSeal,
        total_changes: u64,
    ) -> Result<(), String> {
        let stages = self
            .stages
            .as_ref()
            .ok_or_else(|| stale("缺实际stage封口"))?;
        if !self.closed
            || stages.total_changes != total_changes
            || stages.material_epoch < self.material_epoch
            || stages.stage_count > stages.ordered_events
            || self.seal.life != seal.life
            || self.seal.transaction_id != seal.transaction_id
            || self.seal.data_version != seal.data_version
            || self.seal.schema_version != seal.schema_version
            || self.seal.domain != seal.domain
            || self.seal.write_sequence.checked_add(1) != Some(seal.write_sequence)
        {
            return Err(stale("集合effects未闭合或提交域改变"));
        }
        Ok(())
    }
    pub(super) fn committed(self) -> CommittedSet {
        debug_assert!(self.closed);
        CommittedSet {
            root: self.current,
            raw: self.raw,
            knowledge: self.knowledge,
        }
    }
}
fn knowledge_at(
    s: &mut (impl CodecRead + ?Sized),
    objects: sc::RefView<'_>,
    ordinal: u64,
) -> Result<Option<Knowledge>, String> {
    use sc::path::PathStep;
    let object = s
        .lookup_ref(objects, &[PathStep::Index(ordinal)])
        .map_err(|e| stale(&e.to_string()))?;
    let kind = s
        .lookup_field_ref(object.view(), "kind")
        .map_err(|e| stale(&e.to_string()))?;
    let value = super::super::business_json::render_json(s, kind.view())?;
    if value.view().as_str() != Some(crate::content_producer::CC054_KNOWLEDGE_KIND) {
        return Ok(None);
    }
    let payload = s
        .lookup_field_ref(object.view(), "payload_json")
        .map_err(|e| stale(&e.to_string()))?;
    let inner = s
        .lookup_ref(payload.view(), &[PathStep::JsonText])
        .map_err(|e| stale(&e.to_string()))?;
    let (requests, _) = array(s, inner.view(), "requests")?;
    Ok(Some(Knowledge { ordinal, requests }))
}
/// 只接 prepared 已完成原完整来源判据的同事务材料；不改变原失败首报顺序。
pub(super) fn qualify(
    seed: Seed,
    s: &mut (impl CodecRead + ?Sized),
    current: sc::RefView<'_>,
    material_epoch: u64,
    budget: &native::Budget,
) -> Result<PendingSet, String> {
    if s.allocation_budget().is_some() {
        return Err(stale("集合owner尚无Owned同账入口"));
    }
    let old_ref = seed.previous.map(|r| r.reference(s)).transpose()?;
    let (objects, objects_seq) = array(s, current, "objects")?;
    let (_, witnesses_seq) = array(s, current, "witnesses")?;
    let (_, observations_seq) = array(s, current, "observations")?;
    let mut old_arrays = [None; 3];
    let mut old_objects = None;
    if let Some(old) = &old_ref {
        for (i, key) in ["objects", "witnesses", "observations"].iter().enumerate() {
            let (declaration, sequence) = array(s, old.view(), key)?;
            old_arrays[i] = Some(sequence);
            if i == 0 {
                old_objects = Some(declaration);
            }
        }
    }
    let arrays = [
        difference(s, budget, old_arrays[0], Some(objects_seq))?,
        difference(s, budget, old_arrays[1], Some(witnesses_seq))?,
        difference(s, budget, old_arrays[2], Some(observations_seq))?,
    ];
    let mut previous_knowledge = seed.knowledge;
    if seed.bootstrap {
        if let Some(old) = old_objects {
            let old = old.reference(s)?;
            for ordinal in 0..old.view().extent().ok_or_else(|| stale("旧对象集缺长度"))? {
                if let Some(found) = knowledge_at(s, old.view(), ordinal)? {
                    if previous_knowledge.replace(found).is_some() {
                        return Err(stale("已资格旧材料CC054不唯一"));
                    }
                }
            }
        }
    }
    let objects = objects.reference(s)?;
    let mut knowledge = None;
    for edit in &arrays[0].edits {
        match *edit {
            Edit::Retain { old, current } => {
                if let Some(prior) = previous_knowledge {
                    if prior.ordinal >= old.ordinal
                        && prior.ordinal - old.ordinal < old.sequence.count()
                    {
                        knowledge = Some(Knowledge {
                            ordinal: current.ordinal + prior.ordinal - old.ordinal,
                            requests: prior.requests,
                        });
                    }
                }
            }
            Edit::Add { ordinal, .. } => {
                if let Some(found) = knowledge_at(s, objects.view(), ordinal)? {
                    if knowledge.replace(found).is_some() {
                        return Err(stale("已资格当前材料CC054不唯一"));
                    }
                }
            }
            Edit::Remove { .. } => {}
        }
    }
    let requests = if previous_knowledge.is_some() || knowledge.is_some() {
        let mut seqs = [None; 2];
        for (i, record) in [previous_knowledge, knowledge].into_iter().enumerate() {
            if let Some(record) = record {
                let r = record.requests.reference(s)?;
                seqs[i] = Some(
                    sc::path::array_sequence_refview(s.generic_source(), r.view())
                        .map_err(|e| stale(&e.to_string()))?,
                );
            }
        }
        Some(difference(s, budget, seqs[0], seqs[1])?)
    } else {
        None
    };
    Ok(PendingSet {
        previous: seed.previous,
        current: Declaration::read(current)?,
        raw: seed.raw,
        seal: seed.seal,
        material_epoch,
        arrays,
        requests,
        knowledge,
        stages: None,
        closed: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc::{NodeSource, Value as V};

    fn sequence(nodes: &mut sc::MemoryNodes, values: &[u8]) -> SequenceRef {
        let value = V::Array(
            values
                .iter()
                .map(|v| V::Number(sc::NumberToken::parse(&v.to_string()).unwrap()))
                .collect(),
        );
        let array = sc::write_value(nodes, &value).unwrap().content_ref();
        sc::path::array_sequence_refview(nodes, (&array).into()).unwrap()
    }
    fn expand(nodes: &mut sc::MemoryNodes, span: Span, out: &mut Vec<(u64, NodeDigest)>) {
        match sc::path::sequence_fragment(nodes, span.sequence).unwrap() {
            SequenceFragment::Empty => {}
            SequenceFragment::Leaf(value) => out.push((span.ordinal, value)),
            SequenceFragment::Branch { left, right } => {
                expand(
                    nodes,
                    Span {
                        ordinal: span.ordinal,
                        sequence: left,
                    },
                    out,
                );
                expand(
                    nodes,
                    Span {
                        ordinal: span.ordinal + left.count(),
                        sequence: right,
                    },
                    out,
                );
            }
        }
    }
    #[test]
    fn owner_set_sequence_exhaustive_occurrences_and_order() {
        let inputs: Vec<Vec<u8>> = (0..=5)
            .flat_map(|n| {
                (0..(1usize << n))
                    .map(move |mask| (0..n).map(|bit| ((mask >> bit) & 1) as u8).collect())
            })
            .collect();
        for old in &inputs {
            for new in &inputs {
                let mut nodes = sc::MemoryNodes::default();
                let before = sequence(&mut nodes, old);
                let after = sequence(&mut nodes, new);
                let budget = native::Budget::new(1 << 20, 0).unwrap();
                let delta = difference(&mut nodes, &budget, Some(before), Some(after)).unwrap();
                let mut old_actual = Vec::new();
                let mut new_actual = Vec::new();
                for edit in delta.edits {
                    match edit {
                        Edit::Retain { old, current } => {
                            expand(&mut nodes, old, &mut old_actual);
                            expand(&mut nodes, current, &mut new_actual);
                        }
                        Edit::Remove { ordinal, value } => old_actual.push((ordinal, value)),
                        Edit::Add { ordinal, value } => new_actual.push((ordinal, value)),
                    }
                }
                let mut old_expected = Vec::new();
                let mut new_expected = Vec::new();
                expand(
                    &mut nodes,
                    Span {
                        ordinal: 0,
                        sequence: before,
                    },
                    &mut old_expected,
                );
                expand(
                    &mut nodes,
                    Span {
                        ordinal: 0,
                        sequence: after,
                    },
                    &mut new_expected,
                );
                assert_eq!(old_actual, old_expected, "{old:?} -> {new:?}");
                assert_eq!(new_actual, new_expected, "{old:?} -> {new:?}");
            }
        }
    }
    struct Reads {
        nodes: sc::MemoryNodes,
        reads: usize,
    }
    impl NodeSource for Reads {
        fn read(&mut self, digest: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
            self.reads += 1;
            self.nodes.read(digest)
        }
    }
    impl CodecRead for Reads {
        fn generic_source(&mut self) -> &mut dyn NodeSource {
            self
        }
    }
    #[test]
    fn owner_set_append_skips_shared_prefix_and_preserves_duplicate_occurrence() {
        let mut nodes = sc::MemoryNodes::default();
        let before = sequence(&mut nodes, &vec![7; 4095]);
        let after = sequence(&mut nodes, &vec![7; 4096]);
        let mut reader = Reads { nodes, reads: 0 };
        let budget = native::Budget::new(1 << 20, 0).unwrap();
        let delta = difference(&mut reader, &budget, Some(before), Some(after)).unwrap();
        assert!(reader.reads < 64, "actual node reads={}", reader.reads);
        assert_eq!(
            delta
                .edits
                .iter()
                .filter(|e| matches!(e, Edit::Add { .. }))
                .count(),
            1
        );
        assert!(!delta.edits.iter().any(|e| matches!(e, Edit::Remove { .. })));
        assert!(matches!(
            delta.edits.last(),
            Some(Edit::Add { ordinal: 4095, .. })
        ));
    }
    fn seal(raw: RawAdvanceBinding) -> SnapshotSeal {
        SnapshotSeal {
            life: raw.owner_life(),
            transaction_id: 1,
            write_sequence: 0,
            data_version: 1,
            schema_version: 1,
            domain: [0, 1, 1, 0, 0, 1],
        }
    }
    fn state(
        nodes: &mut sc::MemoryNodes,
        requests: &[u8],
        witnesses: &[u8],
        observations: &[u8],
    ) -> sc::ContentRef {
        let strings = |values: &[u8]| {
            V::Array(
                values
                    .iter()
                    .map(|v| V::Number(sc::NumberToken::parse(&v.to_string()).unwrap()))
                    .collect(),
            )
        };
        let knowledge = V::Object(vec![
            (
                "kind".into(),
                V::String(crate::content_producer::CC054_KNOWLEDGE_KIND.into()),
            ),
            (
                "payload_json".into(),
                V::JsonText(Box::new(sc::JsonValue::Object(
                    [(
                        "requests".into(),
                        sc::JsonValue::Array(
                            requests
                                .iter()
                                .map(|v| {
                                    sc::JsonValue::Number(
                                        sc::NumberToken::parse(&v.to_string()).unwrap(),
                                    )
                                })
                                .collect(),
                        ),
                    )]
                    .into_iter()
                    .collect(),
                ))),
            ),
        ]);
        sc::write_value(
            nodes,
            &V::Object(vec![
                ("objects".into(), V::Array(vec![knowledge])),
                ("witnesses".into(), strings(witnesses)),
                ("observations".into(), strings(observations)),
            ]),
        )
        .unwrap()
        .content_ref()
    }
    #[test]
    fn owner_set_full_material_diff_covers_witness_observation_and_nested_requests() {
        let fixture = super::super::construction_state::RawAdvanceFixture::new();
        let events = [serde_json::json!({"identity_key":"raw","revision":1,"source_coord":"0"})];
        let token = fixture.bind((&events).into(), 0, &[]).unwrap();
        let raw = token.current_binding();
        let mut nodes = sc::MemoryNodes::default();
        let old = state(&mut nodes, &[1, 1], &[1, 2], &[7, 7]);
        let current = state(&mut nodes, &[1, 2, 1], &[1], &[7]);
        // 私有材料fixture只检codec闭包与差异，不冒称CC054业务语言合法。
        sc::verify(&mut nodes, &old).unwrap();
        sc::verify(&mut nodes, &current).unwrap();
        let seed = Seed::new(
            raw,
            seal(raw),
            Some(Declaration::read((&old).into()).unwrap()),
            None,
        )
        .unwrap();
        let budget = native::Budget::new(1 << 20, 0).unwrap();
        let set = qualify(seed, &mut nodes, (&current).into(), 4, &budget).unwrap();
        assert!(set.arrays[1]
            .edits
            .iter()
            .any(|e| matches!(e, Edit::Remove { ordinal: 1, .. })));
        assert!(set.arrays[2]
            .edits
            .iter()
            .any(|e| matches!(e, Edit::Remove { ordinal: 1, .. })));
        let requests = set.requests.as_ref().unwrap();
        assert_eq!(requests.old.unwrap().count(), 2);
        assert_eq!(requests.current.unwrap().count(), 3);
        assert!(requests
            .edits
            .iter()
            .any(|e| matches!(e, Edit::Add { ordinal: 1, .. })));
        assert!(
            set.check_commit(seal(raw), 0).is_err(),
            "材料差异不是提交许可"
        );
    }
    #[test]
    fn owner_set_wrong_root_owner_and_unclosed_domain_reject() {
        let fixture = super::super::construction_state::RawAdvanceFixture::new();
        let events = [serde_json::json!({"identity_key":"raw","revision":1,"source_coord":"0"})];
        let token = fixture.bind((&events).into(), 0, &[]).unwrap();
        let raw = token.current_binding();
        let mut bad = seal(raw);
        bad.life += 1;
        assert!(Seed::new(raw, bad, None, None).is_err());
        let mut nodes = sc::MemoryNodes::default();
        let root = state(&mut nodes, &[1], &[], &[7]);
        let foreign_root = state(&mut nodes, &[2], &[], &[7]);
        let seed = Seed::new(raw, seal(raw), None, None).unwrap();
        let budget = native::Budget::new(1 << 20, 0).unwrap();
        let mut set = qualify(seed, &mut nodes, (&root).into(), 1, &budget).unwrap();
        assert!(set
            .close_against_persisted((&foreign_root).into(), raw)
            .is_err());
        assert!(set.check_commit(seal(raw), 0).is_err());
        set.close_stages(1, 1, 1, 1).unwrap();
        set.close_against_persisted((&root).into(), raw).unwrap();
        let mut closed = seal(raw);
        closed.write_sequence += 1;
        set.check_commit(closed, 1).unwrap();
        let saved = set.committed(); // 状态机单元测试；真实COMMIT另有owner SQL锁。
        assert!(Seed::new(
            raw,
            closed,
            Some(Declaration::read((&foreign_root).into()).unwrap()),
            Some(saved)
        )
        .is_err());
    }
    #[test]
    fn owner_set_diff_budget_failure_does_not_reissue_success() {
        let mut nodes = sc::MemoryNodes::default();
        let before = sequence(&mut nodes, &[1, 1, 1, 1]);
        let after = sequence(&mut nodes, &[2, 2, 2, 2]);
        let budget = native::Budget::new(512, 0).unwrap();
        assert!(difference(&mut nodes, &budget, Some(before), Some(after)).is_err());
    }
}
