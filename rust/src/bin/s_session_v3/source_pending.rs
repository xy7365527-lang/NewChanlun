//! #1394 r3：核发布步骤、谓词与知识请求的原件关系，不重跑解析器或折叠。
use super::*;
use sc::capture_allocation::audit_json::JsonView;
use std::time::Instant;

const BAR: [&str; 5] = ["source_coord", "open", "high", "low", "close"];
pub(super) const STEP: [&str; 12] = [
    "action",
    "incoming",
    "acc_before",
    "acc_after",
    "direction",
    "direction_evidence",
    "comparisons",
    "endpoint_order",
    "contains",
    "high_sources",
    "low_sources",
    "waiting_reasons",
];
const INITIAL: &str = "initial_direction_unsettled";

/// 与源修订摘要同时冻结五字段投影；借用原始账，Owned 不造 serde 值或普通容器。
pub(super) fn raw_bar(
    s: &(impl CodecRead + ?Sized),
    event: JsonView<'_>,
    deadline: Option<Instant>,
    budget: Option<&dyn super::super::audit_budget::Budget>,
) -> Result<NodeDigest, String> {
    let fields = BAR.map(|k| (k, bj::field(event, k)));
    super::super::history::hash_json_in_store(
        s,
        JsonView::object_fields(&fields).map_err(err)?,
        deadline,
        budget,
    )
}

pub(super) struct Step<'a> {
    object: &'a Object,
    fields: SourceFields<'static>,
    key: bj::TextValue,
    reasons: RefValue,
    insufficient: bool,
    incomplete: bool,
}

/// 所有实现均借已核 Object；只抽出存储形状，不抽走任何业务判断。
pub(super) trait StepView {
    fn object(&self) -> &Object;
    fn fields(&self) -> &SourceFields<'static>;
    fn key(&self) -> &bj::TextValue;
    fn reasons(&self) -> &RefValue;
    fn insufficient(&self) -> bool;
    fn incomplete(&self) -> bool;
}
impl StepView for Step<'_> {
    fn object(&self) -> &Object {
        self.object
    }
    fn fields(&self) -> &SourceFields<'static> {
        &self.fields
    }
    fn key(&self) -> &bj::TextValue {
        &self.key
    }
    fn reasons(&self) -> &RefValue {
        &self.reasons
    }
    fn insufficient(&self) -> bool {
        self.insufficient
    }
    fn incomplete(&self) -> bool {
        self.incomplete
    }
}

#[cfg(feature = "s_capture_native_testonly")]
pub(super) struct StepContribution {
    pub(super) object: std::rc::Rc<Object>,
    pub(super) fields: SourceFields<'static>,
    pub(super) key: bj::TextValue,
    pub(super) reasons: RefValue,
    pub(super) insufficient: bool,
    pub(super) incomplete: bool,
}
#[cfg(feature = "s_capture_native_testonly")]
impl StepContribution {
    pub(super) fn bind(
        s: &mut (impl SourceFieldStore + ?Sized),
        object: std::rc::Rc<Object>,
        raw: &(impl RawLookup + ?Sized),
    ) -> Result<(i64, Self), String> {
        let (coord, step) = step_record(s, &object, raw)?;
        let Step {
            object: _,
            fields,
            key,
            reasons,
            insufficient,
            incomplete,
        } = step;
        Ok((
            coord,
            Self {
                object,
                fields,
                key,
                reasons,
                insufficient,
                incomplete,
            },
        ))
    }
}
#[cfg(feature = "s_capture_native_testonly")]
impl StepView for StepContribution {
    fn object(&self) -> &Object {
        &self.object
    }
    fn fields(&self) -> &SourceFields<'static> {
        &self.fields
    }
    fn key(&self) -> &bj::TextValue {
        &self.key
    }
    fn reasons(&self) -> &RefValue {
        &self.reasons
    }
    fn insufficient(&self) -> bool {
        self.insufficient
    }
    fn incomplete(&self) -> bool {
        self.incomplete
    }
}

fn payload(s: &mut (impl CodecRead + ?Sized), object: &Object) -> Result<RefValue, String> {
    let payload = object.fields.get_admitted(s, "payload_json")?;
    s.lookup_ref(payload.view(), &[PathStep::JsonText])
        .map_err(err)
}

fn nulls(
    s: &(impl CodecRead + ?Sized),
    fields: &SourceFields<'_>,
    names: &[&str],
) -> Result<(), String> {
    for name in names {
        if fields.get_admitted(s, name)?.kind() != Kind::Null {
            return Err(err("初始未定/seed步骤出现非空方向或累积结果"));
        }
    }
    Ok(())
}

fn strings(s: &mut (impl CodecRead + ?Sized), r: &RefValue, want: &[&str]) -> Result<(), String> {
    exact(r, Kind::Array, want.len() as u64)?;
    for (item, expected) in elements(s, r)?.iter().zip(want) {
        if text_value(s, item, "数组文本")? != *expected {
            return Err(err("步骤/知识请求的完整文本数组不同"));
        }
    }
    Ok(())
}

fn singleton(s: &mut (impl CodecRead + ?Sized), r: &RefValue, coord: i64) -> Result<(), String> {
    let values = coords(s, r)?;
    if &*values != [coord] {
        return Err(err("seed极值来源不是本cut首来源"));
    }
    Ok(())
}

// 只核 producer 已发布的比较记录与端点分组，不调用 contains/strict_dir/fold。
fn comparisons(
    s: &mut (impl SourceFieldStore + ?Sized),
    step: &SourceFields<'_>,
    seed: &RefValue,
    require_containment: bool,
) -> Result<(), String> {
    let acc = SourceFields::read_subset(s, seed.view(), &["high", "low"])?;
    let incoming = step.get_admitted(s, "incoming")?;
    let incoming = SourceFields::read_subset(s, incoming.view(), &["high", "low"])?;
    let prices = [
        decimal(&acc.text(s, "high")?)?,
        decimal(&acc.text(s, "low")?)?,
        decimal(&incoming.text(s, "high")?)?,
        decimal(&incoming.text(s, "low")?)?,
    ];
    // 核已发表的首 waiting 谓词与同 cut 原件一致，不重跑解析器生成结构。
    // 后续 waiting 的 contains 为 null，不能把后来的非包含行情反折为已知方向。
    if require_containment
        && !((prices[0] >= prices[2] && prices[1] <= prices[3])
            || (prices[2] >= prices[0] && prices[3] <= prices[1]))
    {
        return Err(err("首waiting的真实OHLC不满足包含关系"));
    }
    let list = step.get_admitted(s, "comparisons")?;
    exact(&list, Kind::Array, 2)?;
    for (i, item) in elements(s, &list)?.iter().enumerate() {
        exact(item, Kind::Object, 4)?;
        let fields =
            SourceFields::read_subset(s, item.view(), &["axis", "acc", "incoming", "order"])?;
        let order = match prices[i].cmp(&prices[i + 2]) {
            std::cmp::Ordering::Less => "LT",
            std::cmp::Ordering::Equal => "EQ",
            std::cmp::Ordering::Greater => "GT",
        };
        if fields.text(s, "axis")? != ["high", "low"][i]
            || fields.get_admitted(s, "acc")? != acc.get_admitted(s, ["high", "low"][i])?
            || fields.get_admitted(s, "incoming")?
                != incoming.get_admitted(s, ["high", "low"][i])?
            || fields.text(s, "order")? != order
        {
            return Err(err("waiting比较记录与seed/incoming不同"));
        }
    }
    let mut endpoints = [
        (prices[1], "acc.low"),
        (prices[0], "acc.high"),
        (prices[3], "incoming.low"),
        (prices[2], "incoming.high"),
    ];
    endpoints.sort_unstable();
    let mut offset = 0;
    let endpoint_order = step.get_admitted(s, "endpoint_order")?;
    for class in elements(s, &endpoint_order)? {
        if offset == endpoints.len() {
            return Err(err("waiting端点类多余"));
        }
        let start = offset;
        while offset < endpoints.len() && endpoints[offset].0 == endpoints[start].0 {
            offset += 1;
        }
        exact(&class, Kind::Array, (offset - start) as u64)?;
        for (item, (_, label)) in elements(s, &class)?.iter().zip(&endpoints[start..offset]) {
            if text_value(s, item, "端点")? != *label {
                return Err(err("waiting端点类与原件不同"));
            }
        }
    }
    if offset != endpoints.len() {
        return Err(err("waiting端点类缺失"));
    }
    Ok(())
}

/// 单步骤当前原件核；完整前缀与请求全体性由调用方另核。
pub(super) fn step_record<'a>(
    s: &mut (impl SourceFieldStore + ?Sized),
    object: &'a Object,
    raw: &(impl RawLookup + ?Sized),
) -> Result<(i64, Step<'a>), String> {
    let p = payload(s, object)?;
    exact(&p, Kind::Object, STEP.len() as u64)?;
    let fields = SourceFields::read_subset(s, p.view(), &STEP)?;
    let incoming = fields.get_admitted(s, "incoming")?;
    exact(&incoming, Kind::Object, 5)?;
    let coord = decimal(&text(s, &incoming, "source_coord")?)?;
    let key = object.fields.text(s, "fact_key")?;
    let parsed = parsed_json(s, key.as_bytes())?;
    let key_coord = parsed
        .view()
        .index(1)
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("CC004槽位不是源坐标文本"))?;
    let incoming_digest = digest(&incoming)?;
    if decimal(key_coord)? != coord
        || raw
            .get(&coord)
            .is_none_or(|source| source.bar != incoming_digest)
    {
        return Err(err("CC004 incoming/槽位/OHLC不是同cut原始来源"));
    }
    let reasons = fields.get_admitted(s, "waiting_reasons")?;
    let reason_items = elements(s, &reasons)?;
    let mut insufficient = false;
    for item in &reason_items {
        let reason = text_value(s, item, "waiting reason")?;
        insufficient |= matches!(
            reason.as_str(),
            "fewer_than_three_merged_bars" | "no_subsequent_bar" | "direction_not_yet_established"
        );
    }
    let incomplete = !reason_items.is_empty()
        || fields.get_admitted(s, "direction_evidence")?.kind() == Kind::Null;
    insufficient |= reason_items.is_empty()
        && fields.get_admitted(s, "direction_evidence")?.kind() == Kind::Null;
    drop(parsed);
    Ok((
        coord,
        Step {
            object,
            fields,
            key,
            reasons,
            insufficient,
            incomplete,
        },
    ))
}

pub(super) fn knowledge_record<'a>(
    s: &mut (impl SourceFieldStore + ?Sized),
    object: &'a Object,
    knowledge: &mut Option<(RefValue, &'a Object)>,
) -> Result<(), String> {
    let p = payload(s, object)?;
    let key = object.fields.text(s, "fact_key")?;
    let parsed = parsed_json(s, key.as_bytes())?;
    if parsed.view().index(1).and_then(|v| v.as_str()) != Some("TB-02-A")
        || text(s, &p, "scope")? != "TB-02-A"
        || knowledge.replace((p, object)).is_some()
    {
        return Err(err("同cut CC054知识域重复或不符"));
    }
    Ok(())
}

fn step_sequence_record(
    s: &mut (impl SourceFieldStore + ?Sized),
    step: &Step<'_>,
    coord: &i64,
    ordinal: usize,
    prefix: &[i64],
    seed: &mut Option<RefValue>,
    unsettled: &mut bool,
    pending: &mut sc::CodecVec<i64>,
) -> Result<(), String> {
    step_sequence_record_view(
        s,
        step,
        coord,
        ordinal,
        &*step.object.sources == prefix,
        seed,
        unsettled,
        pending,
    )
}

pub(super) fn step_sequence_record_view(
    s: &mut (impl SourceFieldStore + ?Sized),
    step: &(impl StepView + ?Sized),
    coord: &i64,
    ordinal: usize,
    prefix_equal: bool,
    seed: &mut Option<RefValue>,
    unsettled: &mut bool,
    pending: &mut sc::CodecVec<i64>,
) -> Result<(), String> {
    if !prefix_equal {
        return Err(err("CC004来源不是截至incoming的同cut完整前缀"));
    }
    let fields = step.fields();
    let action = fields.text(s, "action")?;
    if ordinal == 0 {
        if action != "seed"
            || fields.get_admitted(s, "acc_after")? != fields.get_admitted(s, "incoming")?
        {
            return Err(err("首CC004不是原件seed"));
        }
        nulls(
            s,
            fields,
            &["acc_before", "contains", "direction", "direction_evidence"],
        )?;
        for key in ["comparisons", "endpoint_order", "waiting_reasons"] {
            let value = fields.get_admitted(s, key)?;
            strings(s, &value, &[])?;
        }
        for key in ["high_sources", "low_sources"] {
            let value = fields.get_admitted(s, key)?;
            singleton(s, &value, *coord)?;
        }
        *seed = Some(fields.get_admitted(s, "incoming")?);
    } else if action == "waiting" {
        if !*unsettled && ordinal != 1 {
            return Err(err("初始未定不是从首包含对开始"));
        }
        let seed = seed.as_ref().ok_or_else(|| err("waiting缺seed原件"))?;
        if fields.get_admitted(s, "acc_before")? != *seed {
            return Err(err("waiting acc_before不是本cut seed原件"));
        }
        nulls(s, fields, &["acc_after", "direction", "direction_evidence"])?;
        let contains = fields.scalar(s, "contains")?;
        if (!*unsettled && contains.view().as_bool() != Some(true))
            || (*unsettled && !contains.view().is_null())
        {
            return Err(err("waiting首包含/后续未判定记录不同"));
        }
        strings(s, step.reasons(), &[INITIAL])?;
        for key in ["high_sources", "low_sources"] {
            let value = fields.get_admitted(s, key)?;
            strings(s, &value, &[])?;
        }
        comparisons(s, fields, seed, !*unsettled)?;
        s.native_codec_room(pending, 1).map_err(err)?;
        pending.try_push(*coord).map_err(err)?;
        *unsettled = true;
    } else {
        if *unsettled
            || !matches!(
                action.as_str(),
                "merge" | "new_group" | "establish_direction"
            )
        {
            return Err(err("CC004未定后缀中断或步骤action不符"));
        }
        for reason in elements(s, step.reasons())? {
            if text_value(s, &reason, "waiting reason")? == INITIAL {
                return Err(err("非waiting步骤携带初始未定原因"));
            }
        }
    }
    Ok(())
}

pub(super) fn unsettled_seed_group(
    s: &mut (impl SourceFieldStore + ?Sized),
    coord: i64,
    groups: &(impl GroupLookup + ?Sized),
    seed: &Option<RefValue>,
) -> Result<(), String> {
    let group = groups.get(&0).ok_or_else(|| err("初始未定缺seed组"))?;
    if groups.len() != 1
        || &*group.members != [coord]
        || &*group.sources != [coord]
        || group.anchor != coord
    {
        return Err(err("初始未定组不是唯一seed原件"));
    }
    let seed = seed.as_ref().expect("unsettled checked seed");
    for key in ["open", "high", "low", "close"] {
        if field(s, &group.payload, key)? != field(s, seed, key)? {
            return Err(err("初始未定seed组OHLC不同"));
        }
    }
    let group_fields = SourceFields::read_subset(
        s,
        group.payload.view(),
        &[
            "confirmed",
            "direction",
            "direction_evidence",
            "confirmation_evidence",
            "high_sources",
            "low_sources",
            "waiting_reasons",
        ],
    )?;
    nulls(
        s,
        &group_fields,
        &["direction", "direction_evidence", "confirmation_evidence"],
    )?;
    if group_fields.scalar(s, "confirmed")?.view().as_bool() != Some(false) {
        return Err(err("初始未定seed组被声明为已确认"));
    }
    for key in ["high_sources", "low_sources"] {
        let value = group_fields.get_admitted(s, key)?;
        singleton(s, &value, coord)?;
    }
    let waiting_reasons = group_fields.get_admitted(s, "waiting_reasons")?;
    strings(s, &waiting_reasons, &["direction_not_yet_established"])?;
    Ok(())
}

pub(super) trait StepLookup {
    fn get_step(&self, coord: i64) -> Option<&dyn StepView>;
    fn contains_subject(&self, id: &str) -> bool;
}
pub(super) trait RequestSlots {
    fn insert_request(&mut self, s: &(impl CodecRead + ?Sized), key: &str) -> Result<bool, String>;
    fn insert_bound(&mut self, s: &(impl CodecRead + ?Sized), coord: i64) -> Result<bool, String>;
}
struct FullStepLookup<'a, 'b, 'c> {
    steps: &'a BusinessMap<i64, Step<'b>>,
    subjects: &'a BusinessMap<&'c str, i64>,
}
impl StepLookup for FullStepLookup<'_, '_, '_> {
    fn get_step(&self, coord: i64) -> Option<&dyn StepView> {
        self.steps.get(&coord).map(|step| step as &dyn StepView)
    }
    fn contains_subject(&self, id: &str) -> bool {
        self.subjects.contains_key(id)
    }
}
struct FullRequestSlots<'a> {
    requests: &'a mut BusinessMap<bj::TextValue, ()>,
    bound: &'a mut BusinessMap<i64, ()>,
}
impl RequestSlots for FullRequestSlots<'_> {
    fn insert_request(&mut self, s: &(impl CodecRead + ?Sized), key: &str) -> Result<bool, String> {
        s.native_map_room(self.requests).map_err(err)?;
        Ok(self
            .requests
            .insert(s.native_text(key)?, ())
            .map_err(err)?
            .is_some())
    }
    fn insert_bound(&mut self, s: &(impl CodecRead + ?Sized), coord: i64) -> Result<bool, String> {
        s.native_map_room(self.bound).map_err(err)?;
        Ok(self.bound.insert(coord, ()).map_err(err)?.is_some())
    }
}
fn request_record(
    s: &mut (impl SourceFieldStore + ?Sized),
    request: &RefValue,
    steps: &BusinessMap<i64, Step<'_>>,
    step_subjects: &BusinessMap<&str, i64>,
    requests: &mut BusinessMap<bj::TextValue, ()>,
    bound: &mut BusinessMap<i64, ()>,
) -> Result<(), String> {
    request_record_with_slots(
        s,
        request,
        &FullStepLookup {
            steps,
            subjects: step_subjects,
        },
        &mut FullRequestSlots { requests, bound },
    )
}

pub(super) fn request_record_with_slots(
    s: &mut (impl SourceFieldStore + ?Sized),
    request: &RefValue,
    steps: &impl StepLookup,
    slots: &mut impl RequestSlots,
) -> Result<(), String> {
    exact(request, Kind::Object, 5)?;
    let fields = SourceFields::read_subset(
        s,
        request.view(),
        &[
            "request_id",
            "subject_id",
            "axes",
            "reasons",
            "source_coords",
        ],
    )?;
    let key = fields.text(s, "request_id")?;
    let parsed = parsed_json(s, key.as_bytes())?;
    let view = parsed.view();
    if !canonical_matches(s, view, key.as_bytes())? || view.as_array().is_none_or(|a| a.len() != 2)
    {
        return Err(err("知识请求键不是规范二元槽位"));
    }
    if slots.insert_request(s, key.as_str())? {
        return Err(err("同cut知识请求重复"));
    }
    if view.index(0).and_then(|v| v.as_str()) != Some(CC004_STEP_KIND) {
        // 非步骤请求可共存，但不能换一个 key 重复声明同一等待主体。
        let subject = fields.scalar(s, "subject_id")?;
        if subject
            .view()
            .as_str()
            .is_some_and(|id| steps.contains_subject(id))
        {
            return Err(err("CC004知识主体被绑定到其它请求槽位"));
        }
        return Ok(());
    }
    let coord = decimal(
        view.index(1)
            .and_then(|v| v.as_str())
            .ok_or_else(|| err("步骤请求槽位非坐标"))?,
    )?;
    let step = steps
        .get_step(coord)
        .ok_or_else(|| err("多余CC004知识请求"))?;
    if &key != step.key()
        || fields.text(s, "subject_id")? != step.object().id
        || {
            let sources = fields.get_admitted(s, "source_coords")?;
            &*coords(s, &sources)? != &*step.object().sources
        }
        || { slots.insert_bound(s, coord)? }
    {
        return Err(err("CC004知识请求key/subject/来源不一致"));
    }
    let reasons = fields.get_admitted(s, "reasons")?;
    if step.reasons().extent() == Some(0)
        && step.fields().get_admitted(s, "direction_evidence")?.kind() == Kind::Null
    {
        strings(s, &reasons, &["direction_not_yet_established"])?;
    } else if &reasons != step.reasons() {
        return Err(err("CC004知识请求未绑定完整reasons"));
    }
    let axes = fields.get_admitted(s, "axes")?;
    exact(&axes, Kind::Object, 4)?;
    let axes = SourceFields::read_subset(
        s,
        axes.view(),
        &["input_quality", "applicability", "computation", "validity"],
    )?;
    for (name, want) in [
        (
            "input_quality",
            if step.insufficient() {
                "INSUFFICIENT"
            } else {
                "SUFFICIENT"
            },
        ),
        (
            "applicability",
            if step.incomplete() {
                "DOMAIN_PROOF_MISSING"
            } else {
                "IN_DOMAIN"
            },
        ),
        (
            "computation",
            if step.incomplete() {
                "INCOMPLETE"
            } else {
                "COMPLETE"
            },
        ),
        ("validity", "CURRENT"),
    ] {
        if axes.text(s, name)? != want {
            return Err(err("CC004知识请求四轴不同"));
        }
    }
    Ok(())
}

pub(super) fn audit<'a>(
    s: &mut (impl SourceFieldStore + ?Sized),
    objects: impl IntoIterator<Item = &'a Object>,
    raw: &BusinessMap<i64, Source>,
    groups: &BusinessMap<i64, Group>,
    deadline: Option<Instant>,
) -> Result<sc::CodecVec<i64>, String> {
    #[cfg(test)]
    super::source_call_note(|n| {
        n.full_pending_calls += 1;
    });
    let allocation = source_allocation(s)?;
    let mut steps = bj::new_map(allocation.as_ref());
    let mut step_subjects = bj::new_map(allocation.as_ref());
    let mut knowledge = None;
    #[cfg(test)]
    super::source_call_note(|n| n.full_members_enumerations += 1);
    for object in objects {
        super::super::history::check_deadline(deadline)?;
        if object.kind == CC054_KNOWLEDGE_KIND {
            knowledge_record(s, object, &mut knowledge)?;
        } else if object.kind == CC004_STEP_KIND {
            let (coord, step) = step_record(s, object, raw)?;
            s.native_map_room(&mut step_subjects).map_err(err)?;
            step_subjects
                .insert(object.id.as_str(), coord)
                .map_err(err)?;
            s.native_map_room(&mut steps).map_err(err)?;
            if steps.insert(coord, step).map_err(err)?.is_some() {
                return Err(err("同cut CC004步骤坐标重复"));
            }
        }
    }
    if steps.len() != raw.len() {
        return Err(err("同cut CC004未一一覆盖生效原始账"));
    }
    #[cfg(test)]
    super::source_call_note(|n| n.full_members_enumerations += 1);
    let ordered = sorted_numeric_entries(s, raw)?;
    let mut prefix = bj::new_work(allocation.as_ref());
    let mut pending = bj::new_work(allocation.as_ref());
    let mut seed = None;
    let mut unsettled = false;
    for (ordinal, (coord, _)) in ordered.iter().enumerate() {
        super::super::history::check_deadline(deadline)?;
        s.native_codec_room(&mut prefix, 1).map_err(err)?;
        prefix.try_push(*coord).map_err(err)?;
        let step = steps.get(coord).ok_or_else(|| err("raw缺当前CC004步骤"))?;
        step_sequence_record(
            s,
            step,
            coord,
            ordinal,
            &prefix,
            &mut seed,
            &mut unsettled,
            &mut pending,
        )?;
    }
    if unsettled {
        unsettled_seed_group(s, ordered[0].0, groups, &seed)?;
    }
    // 全部请求只读一遍，以具名step键绑定；其它形态/组请求合法共存。
    let (knowledge, knowledge_object) = knowledge.ok_or_else(|| err("同cut缺CC054知识原件"))?;
    if &*knowledge_object.sources != &*prefix {
        return Err(err("CC054来源不是同cut全部生效原始账"));
    }
    let mut requests = bj::new_map(allocation.as_ref());
    let mut bound = bj::new_map(allocation.as_ref());
    let request_array = field(s, &knowledge, "requests")?;
    #[cfg(test)]
    super::source_call_note(|n| n.full_members_enumerations += 1);
    for request in elements(s, &request_array)? {
        super::super::history::check_deadline(deadline)?;
        request_record(
            s,
            &request,
            &steps,
            &step_subjects,
            &mut requests,
            &mut bound,
        )?;
    }
    if bound.len() != steps.len() {
        return Err(err("CC004缺一一绑定的知识请求"));
    }
    Ok(pending)
}
