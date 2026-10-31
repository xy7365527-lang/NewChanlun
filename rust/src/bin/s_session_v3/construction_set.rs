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
    pub(super) fn built(self) -> sc::Built {
        sc::Built {
            digest: self.digest,
            kind: self.kind,
            extent: self.extent,
        }
    }
    pub(super) fn read(reference: sc::RefView<'_>) -> Result<Self, String> {
        Ok(Self {
            digest: reference.digest().map_err(|e| stale(&e.to_string()))?,
            kind: reference.kind(),
            extent: reference.extent(),
        })
    }
    pub(super) fn reference(
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
pub(crate) struct Span {
    pub(crate) ordinal: u64,
    pub(crate) sequence: SequenceRef,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edit {
    Retain { old: Span, current: Span },
    Remove { ordinal: u64, value: NodeDigest },
    Add { ordinal: u64, value: NodeDigest },
}
pub(crate) struct SequenceDelta {
    pub(crate) old: Option<SequenceRef>,
    pub(crate) current: Option<SequenceRef>,
    pub(crate) edits: native::Reserved<Vec<Edit>>,
}
impl SequenceDelta {
    fn retained_bytes(&self) -> usize {
        vec_bytes::<Edit>(self.edits.capacity())
    }
}
fn grow<T>(b: &native::Budget, v: &mut native::Reserved<Vec<T>>, value: T) -> Result<(), String> {
    b.vec_grow(v, 1).map_err(|_| stale("集合差异额度不足"))?;
    v.push(value);
    Ok(())
}
fn split(
    s: &mut (impl CodecRead + ?Sized),
    b: &native::Budget,
    stack: &mut native::Reserved<Vec<Span>>,
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
    let mut before = b.empty().map_err(|_| stale("集合差异许可不足"))?;
    let mut after = b.empty().map_err(|_| stale("集合差异许可不足"))?;
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
    let mut edits = b.empty().map_err(|_| stale("集合差异许可不足"))?;
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
    drop((before, after));
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
    #[allow(dead_code, reason = "#1488：8组合探针无读取，保留来源状态")]
    pub(super) previous: Option<Declaration>,
    pub(super) current: Declaration,
    pub(super) raw: RawAdvanceBinding,
    pub(super) seal: SnapshotSeal,
    pub(super) material_epoch: u64,
    pub(super) arrays: [SequenceDelta; 3],
    pub(super) requests: Option<SequenceDelta>,
    knowledge: Option<Knowledge>,
    source_check: Option<OwnerSourceCheck>,
    stages: Option<StageSeal>,
    closed: bool,
    // 字段最后析构：差异Vec先真实释放，再归还原work父账的C2 cap许可。
    resource_hold: Option<super::native::Budget>,
    parent_charge: Option<sc::capture_allocation::scratch::NativeReservation>,
}
/// 只由实际S成功结果构造；原完整投影核可复用其来源counts，不能跳过其它判据。
pub(crate) struct OwnerSourceCheck {
    root: Declaration,
    raw_prefix: usize,
    counts: super::super::source_audit::Counts,
    context: Option<crate::content_manifest::NativeMetadata>,
    deadline: Option<std::time::Instant>,
}
#[cfg(test)]
thread_local! {
    static OWNER_SOURCE_REUSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
thread_local! { static SOURCE_CONTEXT_NEGATIVES: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
#[cfg(test)]
pub(super) fn with_source_context_negatives_for_test(f: impl FnOnce()) {
    SOURCE_CONTEXT_NEGATIVES.with(|flag| flag.set(true));
    f();
    SOURCE_CONTEXT_NEGATIVES.with(|flag| flag.set(false));
}
#[cfg(test)]
pub(super) fn take_owner_source_reuses() -> usize {
    OWNER_SOURCE_REUSES.with(|count| count.replace(0))
}
impl OwnerSourceCheck {
    /// 只能经实际effects完整封口取得本对象；Counts只供当前cut的catalog对拍。
    pub(crate) fn counts_for_cut(
        &self,
        metadata: crate::content_manifest::MetadataView<'_>,
        root: sc::RefView<'_>,
        raw_prefix: usize,
        deadline: Option<std::time::Instant>,
    ) -> Result<super::super::source_audit::Counts, String> {
        super::super::history::check_deadline(self.deadline)?;
        super::super::history::check_deadline(deadline)?;
        if !self
            .context
            .as_ref()
            .is_some_and(|context| context.matches(metadata))
            || usize::try_from(metadata.input_frontier)
                .ok()
                .and_then(|n| n.checked_add(1))
                != Some(raw_prefix)
        {
            return Err(stale("来源成功结果与完整cut请求声明不同"));
        }
        self.counts_for(root, raw_prefix)
    }
    pub(crate) fn counts_for(
        &self,
        root: sc::RefView<'_>,
        raw_prefix: usize,
    ) -> Result<super::super::source_audit::Counts, String> {
        if Declaration::read(root)? != self.root || raw_prefix != self.raw_prefix {
            return Err(stale("来源成功结果与实际持久cut/raw前沿不同"));
        }
        #[cfg(test)]
        OWNER_SOURCE_REUSES.with(|count| count.set(count.get() + 1));
        Ok(self.counts.clone())
    }
}
struct StageSeal {
    material_epoch: u64,
    ordered_events: usize,
    stage_count: usize,
    total_changes: u64,
}
impl PendingSet {
    pub(super) fn bind_source_context(
        &mut self,
        metadata: crate::content_manifest::NativeMetadata,
        deadline: Option<std::time::Instant>,
    ) -> Result<(), String> {
        let source = self
            .source_check
            .as_mut()
            .ok_or_else(|| stale("来源未成功不能绑定请求"))?;
        if source.context.is_some() || self.stages.is_some() || self.closed {
            return Err(stale("来源请求绑定重复或越过stage边界"));
        }
        source.context = Some(metadata);
        source.deadline = deadline;
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn context_negatives_for_test(
        &mut self,
        metadata: &crate::content_manifest::ManifestMetadata,
        root: sc::RefView<'_>,
        raw_prefix: usize,
    ) {
        if !SOURCE_CONTEXT_NEGATIVES.with(|flag| flag.get()) {
            return;
        }
        let source = self.source_check.as_mut().unwrap();
        for changed in 0..7 {
            let mut wrong = metadata.clone();
            match changed {
                0 => wrong.generation += 1,
                1 => wrong.profile_id.push_str("-other"),
                2 => wrong.rule_revision.push_str("-other"),
                3 => wrong.source_commit.message_id.push_str("-other"),
                4 => wrong.writer_epoch.push_str("-other"),
                5 => wrong.scope.extent = Some(u64::MAX),
                _ => wrong.input_frontier += 1,
            }
            assert!(source
                .counts_for_cut(wrong.view(), root, raw_prefix, None)
                .is_err());
        }
        let original = source.deadline;
        source.deadline = Some(std::time::Instant::now());
        assert!(source
            .counts_for_cut(metadata.view(), root, raw_prefix, None)
            .unwrap_err()
            .contains("VerifyDeadline"));
        source.deadline = original;
    }
    pub(super) fn requires_source(&self) -> bool {
        self.source_check.is_some()
    }
    pub(super) fn source_check(&self) -> Result<Option<&OwnerSourceCheck>, String> {
        if self.source_check.is_some() && self.stages.is_none() {
            return Err(stale("来源结果尚无完整实际stage封口"));
        }
        Ok(self.source_check.as_ref())
    }
    pub(super) fn hold_parent_charge(
        &mut self,
        charge: Option<sc::capture_allocation::scratch::NativeReservation>,
    ) -> Result<(), String> {
        if self.parent_charge.is_some() {
            return Err(stale("集合父账许可重复移交"));
        }
        self.parent_charge = charge;
        Ok(())
    }
    pub(super) fn hold_resource_budget(
        &mut self,
        budget: super::native::Budget,
    ) -> Result<(), String> {
        if self.resource_hold.is_some() {
            return Err(stale("集合资源父账重复移交"));
        }
        self.resource_hold = Some(budget);
        Ok(())
    }
    pub(super) fn retained_bytes(&self) -> usize {
        self.arrays
            .iter()
            .map(SequenceDelta::retained_bytes)
            .sum::<usize>()
            + self
                .requests
                .as_ref()
                .map_or(0, SequenceDelta::retained_bytes)
            + self
                .source_check
                .as_ref()
                .and_then(|source| source.context.as_ref())
                .map_or(0, crate::content_manifest::NativeMetadata::retained_bytes)
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
        source_check: None,
        stages: None,
        closed: false,
        parent_charge: None,
        resource_hold: None,
    })
}

/// 原材料资格建立后才构造；只在来源核走到相应阶段时读取该数组。
#[derive(Clone, Copy, Debug)]
pub(crate) enum SourceArray {
    Objects,
    Witnesses,
    Observations,
}
impl SourceArray {
    fn index(self) -> usize {
        match self {
            Self::Objects => 0,
            Self::Witnesses => 1,
            Self::Observations => 2,
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::Objects => "objects",
            Self::Witnesses => "witnesses",
            Self::Observations => "observations",
        }
    }
}
pub(super) struct LazySet {
    seed: Seed,
    current: Declaration,
    arrays: [Option<SequenceDelta>; 3],
    members: [Option<Declaration>; 3],
    requests: Option<SequenceDelta>,
    knowledge: Option<Knowledge>,
}
impl LazySet {
    pub(super) fn new(
        mut seed: Seed,
        current: sc::RefView<'_>,
        previous: Option<&super::super::source_audit::NativeSourceState>,
    ) -> Result<Self, String> {
        if previous.is_none() {
            seed.bootstrap = true;
        }
        Ok(Self {
            seed,
            current: Declaration::read(current)?,
            arrays: [None, None, None],
            members: [None, None, None],
            requests: None,
            knowledge: None,
        })
    }
    pub(super) fn delta<'a>(
        &'a mut self,
        s: &mut (impl CodecRead + ?Sized),
        budget: &native::Budget,
        which: SourceArray,
    ) -> Result<&'a SequenceDelta, String> {
        let i = which.index();
        if self.arrays[i].is_none() {
            let current = self.current.reference(s)?;
            let (declaration, sequence) = array(s, current.view(), which.key())?;
            // 没有前次来源贡献时必须冷建，不能跳过语义上尚未知的共享材料。
            let old = if self.seed.bootstrap {
                None
            } else {
                self.seed
                    .previous
                    .map(|prior| {
                        let r = prior.reference(s)?;
                        array(s, r.view(), which.key()).map(|(_, seq)| seq)
                    })
                    .transpose()?
            };
            self.members[i] = Some(declaration);
            self.arrays[i] = Some(difference(s, budget, old, Some(sequence))?);
        }
        Ok(self.arrays[i].as_ref().unwrap())
    }
    pub(super) fn member(
        &mut self,
        s: &mut (impl CodecRead + ?Sized),
        budget: &native::Budget,
        which: SourceArray,
        ordinal: u64,
    ) -> Result<super::super::audit_store::RefValue, String> {
        self.delta(s, budget, which)?;
        let array = self.members[which.index()].unwrap().reference(s)?;
        s.lookup_ref(array.view(), &[sc::path::PathStep::Index(ordinal)])
            .map_err(|e| stale(&e.to_string()))
    }
    pub(super) fn request_delta<'a>(
        &'a mut self,
        s: &mut (impl CodecRead + ?Sized),
        budget: &native::Budget,
        ordinal: u64,
    ) -> Result<&'a SequenceDelta, String> {
        self.delta(s, budget, SourceArray::Objects)?;
        if let Some(known) = self.knowledge {
            if known.ordinal != ordinal {
                return Err(stale("requests重复绑定不同CC054 ordinal"));
            }
        }
        if self.requests.is_none() {
            let objects = self.members[0].unwrap().reference(s)?;
            let found = knowledge_at(s, objects.view(), ordinal)?
                .ok_or_else(|| stale("requests ordinal不是当前CC054"))?;
            let r = found.requests.reference(s)?;
            let current = sc::path::array_sequence_refview(s.generic_source(), r.view())
                .map_err(|e| stale(&e.to_string()))?;
            let old = if self.seed.bootstrap {
                None
            } else {
                self.seed
                    .knowledge
                    .map(|prior| {
                        let r = prior.requests.reference(s)?;
                        sc::path::array_sequence_refview(s.generic_source(), r.view())
                            .map_err(|e| stale(&e.to_string()))
                    })
                    .transpose()?
            };
            self.requests = Some(difference(s, budget, old, Some(current))?);
            self.knowledge = Some(found);
        }
        Ok(self.requests.as_ref().unwrap())
    }
    pub(super) fn request_member(
        &mut self,
        s: &mut (impl CodecRead + ?Sized),
        budget: &native::Budget,
        knowledge_ordinal: u64,
        ordinal: u64,
    ) -> Result<super::super::audit_store::RefValue, String> {
        self.request_delta(s, budget, knowledge_ordinal)?;
        let requests = self.knowledge.unwrap().requests.reference(s)?;
        s.lookup_ref(requests.view(), &[sc::path::PathStep::Index(ordinal)])
            .map_err(|e| stale(&e.to_string()))
    }
    pub(super) fn finish(
        self,
        material_epoch: u64,
        counts: super::super::source_audit::Counts,
        raw_prefix: usize,
    ) -> Result<PendingSet, String> {
        let [objects, witnesses, observations] = self.arrays;
        Ok(PendingSet {
            previous: self.seed.previous,
            current: self.current,
            raw: self.seed.raw,
            seal: self.seed.seal,
            material_epoch,
            arrays: [
                objects.ok_or_else(|| stale("来源核缺objects阶段"))?,
                witnesses.ok_or_else(|| stale("来源核缺witnesses阶段"))?,
                observations.ok_or_else(|| stale("来源核缺observations阶段"))?,
            ],
            requests: self.requests,
            knowledge: self.knowledge,
            source_check: Some(OwnerSourceCheck {
                root: self.current,
                raw_prefix,
                counts,
                context: None,
                deadline: None,
            }),
            stages: None,
            closed: false,
            parent_charge: None,
            resource_hold: None,
        })
    }
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
    // NOT_RUN：本轮只静态检查。
    fn sequence_delta_retains_charge_and_drop_does_not_refill_next_request() {
        let mut nodes = sc::MemoryNodes::default();
        let before = sequence(&mut nodes, &[1, 2, 3]);
        let after = sequence(&mut nodes, &[1, 3, 4, 5]);
        let budget = native::Budget::new(1 << 20, 0).unwrap();
        let budget = budget.bind_initial(50, 1 << 20).unwrap();
        let base = budget.live();
        let delta = difference(&mut nodes, &budget, Some(before), Some(after)).unwrap();
        assert!(delta
            .edits
            .iter()
            .any(|edit| matches!(edit, Edit::Add { .. })));
        assert_eq!(budget.live(), base + delta.retained_bytes());
        let next = budget.handoff(51, 64).unwrap();
        let all = next.reserve(64).unwrap();
        drop(delta);
        assert_eq!(next.live(), base + 64);
        assert_eq!(next.remaining(), 0);
        drop(all);
        assert_eq!(next.live(), base);
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
