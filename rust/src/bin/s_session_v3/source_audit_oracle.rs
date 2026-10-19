//! #1488：6e38 原完整来源 oracle；仅 cfg(test)，不调用新局部核。
use super::*;
#[path = "source_pending_oracle.rs"]
mod source_pending;
pub(super) fn audit_generated_in_store(
    s: &mut super::super::audit_store::StoreView<'_, '_>,
    state: sc::RefView<'_>,
    raw_prefix: RawEventsView<'_>,
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::super::audit_budget::Budget>,
    mut native_identity: Option<&mut NativeWitnessIdentityMemo<'_>>,
    mut native_sources: Option<&mut NativeSourceMemo<'_>>,
    native_fields: Option<&mut NativeFieldMemo<'_>>,
    native_ledger: Option<&mut NativeRawLedger<'_>>,
) -> Result<Counts, String> {
    super::super::history::check_deadline(deadline)?;
    #[cfg(test)]
    super::super::state::trace_event("source_closure", None);
    s.verify_ref(state, sc::Mode::Public).map_err(err)?;
    let mut array_source = ArrayAuditSource {
        source: s,
        full_arrays: budget.is_none(),
        deadline,
        fields: native_fields,
    };
    let s = &mut array_source;
    #[cfg(test)]
    super::super::state::trace_event("source_raw", None);
    if profile != tb02a::PROFILE {
        return Err(err(
            "当前来源审计仅实现OHLC档；其他档仍待接线，拒绝按此口径放行",
        ));
    }
    s.native_admit(
        profile
            .len()
            .checked_add(rule.len())
            .and_then(|n| n.checked_add(2 * std::mem::size_of::<[usize; 2]>()))
            .ok_or_else(|| err(s.native_overflow()))?,
    )
    .map_err(err)?;
    let producer = Cc004Producer::new(profile, rule);
    let allocation;
    let ordinary_effective;
    let mut ordinary_raw;
    let raw = if let Some(ledger) = native_ledger {
        allocation = source_allocation(s)?;
        ledger.advance(s, raw_prefix.len())?
    } else {
        ordinary_effective = bj::effective(raw_prefix, deadline)?;
        allocation = source_allocation(s)?;
        ordinary_raw = bj::new_map(allocation.as_ref());
        for event in ordinary_effective.iter() {
            super::super::history::check_deadline(deadline)?;
            let coord = bj::coord(event)?;
            let wire = bj::RawReference::new(event, false)?;
            let numeric = bj::RawReference::new(event, true)?;
            s.native_map_room(&mut ordinary_raw).map_err(err)?;
            ordinary_raw
                .insert(
                    coord,
                    Source {
                        wire: super::super::history::hash_json_in_store(
                            s,
                            wire.view(),
                            deadline,
                            budget,
                        )?,
                        numeric: super::super::history::hash_json_in_store(
                            s,
                            numeric.view(),
                            deadline,
                            budget,
                        )?,
                        raw: super::super::history::hash_json_in_store(s, event, deadline, budget)?,
                        bar: source_pending::raw_bar(s, event, deadline, budget)?,
                    },
                )
                .map_err(err)?;
        }
        &ordinary_raw
    };
    let object_array = field_ref(s, state, "objects")?;
    #[cfg(test)]
    super::super::state::trace_event("source_objects", None);
    // 活期只覆盖当前一次audit_generated；不在代际、事务或源修订之间带出。
    let mut checked_refs = bj::new_map(allocation.as_ref());
    let mut objects = bj::new_work(allocation.as_ref());
    let mut ids = bj::new_map(allocation.as_ref());
    let mut groups = bj::new_map(allocation.as_ref());
    for record in elements_ref(s, object_array.view())? {
        super::super::history::check_deadline(deadline)?;
        let kind = if s.fields.is_some() {
            SourceFields::read_subset(s, record.view(), &["kind"])?.text(s, "kind")?
        } else {
            text(s, &record, "kind")?
        };
        let names: &[&'static str] = if kind == CC006_SHAPE_KIND {
            &SHAPE_SOURCE_FIELDS
        } else {
            &TYPED_SOURCE_FIELDS
        };
        let fields = SourceFields::read_subset(s, record.view(), names).map_err(err)?;
        let id = fields.text(s, "object_id").map_err(err)?;
        s.native_map_room(&mut ids).map_err(err)?;
        if ids
            .insert(s.native_text(id.as_str())?, ())
            .map_err(err)?
            .is_some()
            || fields.integer(s, "object_revision").map_err(err)? != 1
        {
            return Err(err("对象身份重复或修订不符"));
        }
        let refs = s
            .lookup_ref(
                fields
                    .get_admitted(s, "input_refs_json")
                    .map_err(err)?
                    .view(),
                &[PathStep::JsonText],
            )
            .map_err(err)?;
        let source_ref = s
            .lookup_ref(
                fields
                    .get_admitted(s, "source_coords_json")
                    .map_err(err)?
                    .view(),
                &[PathStep::JsonText],
            )
            .map_err(err)?;
        let sources = source_coords(
            s,
            &source_ref,
            native_sources.as_deref_mut(),
            ParseDomain::JsonInner,
        )?;
        if kind == CC006_SHAPE_KIND {
            exact(&record, Kind::Object, 15)?;
        } else {
            exact(&record, Kind::Object, 10)?;
            if ![
                CC004_STEP_KIND,
                CC005_GROUP_KIND,
                CC007_FRACTAL_KIND,
                CC054_KNOWLEDGE_KIND,
            ]
            .contains(&kind.as_str())
                && !tb02b::KINDS.contains(&kind.as_str())
            {
                return Err(err("未声明typed事实种类"));
            }
            let payload = s
                .lookup_ref(
                    fields.get_admitted(s, "payload_json").map_err(err)?.view(),
                    &[PathStep::JsonText],
                )
                .map_err(err)?;
            if payload.kind() != Kind::Object {
                return Err(err("typed载荷不是对象"));
            }
            if source_references(
                s,
                &refs,
                &raw,
                false,
                &mut checked_refs,
                native_sources.as_deref_mut(),
                raw_prefix.len(),
                ParseDomain::JsonInner,
            )? != sources
            {
                return Err(err("typed引用与source_coords不同"));
            }
            let key = fields.text(s, "fact_key").map_err(err)?;
            let parsed_owner = parsed_json(s, key.as_bytes())?;
            let parsed = parsed_owner.view();
            if !canonical_matches(s, parsed, key.as_bytes())?
                || parsed.as_array().is_none_or(|a| a.len() != 2)
                || parsed
                    .index(0)
                    .unwrap_or(sc::capture_allocation::audit_json::JsonView::Null)
                    .as_str()
                    != Some(kind.as_str())
            {
                return Err(err("typed槽位种类/规范文本不符"));
            }
            let identity = producer
                .typed_identity_scoped(
                    &kind,
                    &key,
                    &built(payload.view())?,
                    &built(refs.view())?,
                    s.codec_work(),
                )
                .map_err(err)?;
            let expected = s
                .native_format(format_args!(
                    "{}",
                    content_producer::object_id_display(&identity)
                ))
                .map_err(err)?;
            if id != expected {
                return Err(err("typed obj3身份与来源/载荷原件不符"));
            }
            if tb02b::KINDS.contains(&kind.as_str()) {
                for (k, want) in [
                    ("schema_revision", "s-new-bi/1"),
                    (
                        "semantic_version",
                        newchan_rust::theta_v0::parser::stroke::RULE,
                    ),
                    ("policy_version", "standard_raw_gap_3"),
                    ("view_role", "main"),
                ] {
                    if text(s, &payload, k)? != want {
                        return Err(err("新笔载荷语义头不同"));
                    }
                }
                if digest(&field(s, &payload, "slot")?)?
                    != super::super::history::hash_json_in_store(
                        s,
                        parsed
                            .index(1)
                            .unwrap_or(sc::capture_allocation::audit_json::JsonView::Null),
                        deadline,
                        budget,
                    )?
                {
                    return Err(err("新笔payload槽位与fact_key不同"));
                }
            }
            if kind == CC005_GROUP_KIND {
                let index = decimal(&text(s, &payload, "group_index")?)?;
                let anchor = decimal(&text(s, &payload, "group_anchor")?)?;
                let members_root = field(s, &payload, "members")?;
                let members = source_coords(
                    s,
                    &members_root,
                    native_sources.as_deref_mut(),
                    ParseDomain::JsonInner,
                )?;
                if index < 0
                    || members.is_empty()
                    || !members.contains(&anchor)
                    || members.iter().any(|c| !raw.contains_key(c))
                {
                    return Err(err("组成员/锚/序号不符"));
                }
                let mut union = copy_coords(s, &members)?;
                for key in ["direction_evidence", "confirmation_evidence"] {
                    super::super::history::check_deadline(deadline)?;
                    let evidence = field(s, &payload, key)?;
                    if evidence.kind() != Kind::Null {
                        let list = field(s, &evidence, "source_coords")?;
                        let evidence_coords = source_coords(
                            s,
                            &list,
                            native_sources.as_deref_mut(),
                            ParseDomain::JsonInner,
                        )?;
                        union = union_coords(s, &union, &evidence_coords)?;
                    }
                }
                if &*union != &*sources {
                    return Err(err("组来源不等于成员及方向/封口依据的并集"));
                }
                s.native_map_room(&mut groups).map_err(err)?;
                if groups
                    .insert(
                        index,
                        Group {
                            payload: s.native_ref_clone(&payload).map_err(err)?,
                            members,
                            sources: copy_source_coords(s, &sources)?,
                            anchor,
                        },
                    )
                    .map_err(err)?
                    .is_some()
                {
                    return Err(err("同cut组合并序号重复"));
                }
            }
        }
        s.native_codec_room(&mut objects, 1).map_err(err)?;
        objects
            .try_push(Object {
                id,
                kind,
                fields,
                refs,
                sources,
            })
            .map_err(err)?;
    }
    // #1394 r3：先绑定已发布未定事实，再核组成员与 pending 的不交完备并集。
    let pending = source_pending::audit(s, &objects, raw, &groups, deadline)?;
    let mut partition = bj::new_map(allocation.as_ref());
    let ordered_groups = sorted_numeric_entries(s, &groups)?;
    for (expected, (index, group)) in ordered_groups.iter().enumerate() {
        super::super::history::check_deadline(deadline)?;
        if *index != expected as i64 {
            return Err(err("组合并序号不连续"));
        }
        for c in &group.members {
            super::super::history::check_deadline(deadline)?;
            s.native_map_room(&mut partition).map_err(err)?;
            if partition.insert(*c, ()).map_err(err)?.is_some() {
                return Err(err("同cut组成员重叠"));
            }
        }
    }
    drop(ordered_groups);
    for coord in pending {
        super::super::history::check_deadline(deadline)?;
        s.native_map_room(&mut partition).map_err(err)?;
        if partition.insert(coord, ()).map_err(err)?.is_some() {
            return Err(err("同cut组成员与初始未定来源重叠"));
        }
    }
    if partition.len() != raw.len() || partition.iter().any(|(coord, _)| !raw.contains_key(coord)) {
        return Err(err("同cut组未完整覆盖生效原始账"));
    }
    let mut shapes = bj::new_map(allocation.as_ref());
    #[cfg(test)]
    super::super::state::trace_event("source_shapes", None);
    for object in objects.iter().filter(|o| o.kind == CC006_SHAPE_KIND) {
        super::super::history::check_deadline(deadline)?;
        exact(&object.refs, Kind::Array, 3)?;
        let mut indices = [0; 3];
        let mut highs: [bj::TextValue; 3] =
            std::array::from_fn(|_| bj::TextValue::Plain(String::new()));
        let mut lows: [bj::TextValue; 3] =
            std::array::from_fn(|_| bj::TextValue::Plain(String::new()));
        let mut windows =
            std::array::from_fn::<_, 3, _>(|_| crate::content_manifest::Decimal::new(0));
        let mut source_union = bj::new_work(allocation.as_ref());
        for (slot, part) in elements(s, &object.refs)?.into_iter().enumerate() {
            super::super::history::check_deadline(deadline)?;
            exact(&part, Kind::Object, 3)?;
            let part_fields =
                local_fields(s, &part, &["merged_index", "raw_refs", "dependency_refs"])?;
            let index = if let Some(fields) = &part_fields {
                fields
                    .scalar(s, "merged_index")?
                    .view()
                    .as_i64()
                    .ok_or_else(|| err("merged_index不是整数"))?
            } else {
                number(s, &part, "merged_index")?
            };
            indices[slot] = index;
            if slot > 0 && index != indices[slot - 1] + 1 {
                return Err(err("形态来源组不是连续三组"));
            }
            let group = groups
                .get(&index)
                .ok_or_else(|| err("形态引用没有同cut组事实"))?;
            let raw_refs = if let Some(fields) = &part_fields {
                fields.get_admitted(s, "raw_refs").map_err(err)?
            } else {
                field(s, &part, "raw_refs")?
            };
            let members = source_references(
                s,
                &raw_refs,
                &raw,
                true,
                &mut checked_refs,
                native_sources.as_deref_mut(),
                raw_prefix.len(),
                ParseDomain::JsonInner,
            )?;
            if members != group.members {
                return Err(err("形态raw_refs不是同cut组成员"));
            }
            let deps = if let Some(fields) = &part_fields {
                fields.get_admitted(s, "dependency_refs").map_err(err)?
            } else {
                field(s, &part, "dependency_refs")?
            };
            let dependency = source_references(
                s,
                &deps,
                &raw,
                true,
                &mut checked_refs,
                native_sources.as_deref_mut(),
                raw_prefix.len(),
                ParseDomain::JsonInner,
            )?;
            let expected = difference_coords(s, &group.sources, &members)?;
            if &*dependency != &*expected {
                return Err(err("形态dependency_refs不是组事实来源差集"));
            }
            source_union = union_coords(s, &source_union, &members)?;
            source_union = union_coords(s, &source_union, &dependency)?;
            windows[slot] = crate::content_manifest::Decimal::new(
                object
                    .fields
                    .integer(s, ["window_start", "window_mid", "window_end"][slot])
                    .map_err(err)?,
            );
            if windows[slot].as_str()
                != crate::content_manifest::Decimal::new(group.anchor).as_str()
            {
                return Err(err("形态窗口不等于同cut组锚"));
            }
            if let Some(fields) = local_fields(s, &group.payload, &["high", "low"])? {
                highs[slot] = fields.text(s, "high")?;
                lows[slot] = fields.text(s, "low")?;
            } else {
                highs[slot] = text(s, &group.payload, "high")?;
                lows[slot] = text(s, &group.payload, "low")?;
            }
        }
        if &*source_union != &*object.sources {
            return Err(err("形态source_coords不是完整来源并集"));
        }
        let branch = object.fields.text(s, "branch").map_err(err)?;
        let dir_ab = object.fields.text(s, "dir_ab").map_err(err)?;
        let dir_bc = object.fields.text(s, "dir_bc").map_err(err)?;
        let comparison_text = object.fields.text(s, "comparisons_json").map_err(err)?;
        let comparisons_owner = parsed_json(s, comparison_text.as_bytes())?;
        let comparisons = comparisons_owner.view();
        if !canonical_matches(s, comparisons, comparison_text.as_bytes())?
            || comparisons.as_array().is_none_or(|v| v.len() != 4)
        {
            return Err(err("形态比较不是规范四项"));
        }
        let comparisons = Built {
            digest: super::super::history::hash_json_in_store(s, comparisons, deadline, budget)?,
            kind: Kind::Array,
            extent: Some(4),
        };
        let input_refs = built(object.refs.view())?;
        let identity = producer
            .shape_identity_scoped(
                ShapeIdentityView {
                    branch: &branch,
                    dir_ab: &dir_ab,
                    dir_bc: &dir_bc,
                    window: windows.each_ref().map(|v| v.as_str()),
                    highs: highs.each_ref().map(|v| v.as_str()),
                    lows: lows.each_ref().map(|v| v.as_str()),
                    comparisons: &comparisons,
                    input_refs: &input_refs,
                },
                s.codec_work(),
            )
            .map_err(err)?;
        let expected = s
            .native_format(format_args!(
                "{}",
                content_producer::object_id_display(&identity)
            ))
            .map_err(err)?;
        if expected != object.id {
            return Err(err("CC006 obj3与同cut组事实身份不符"));
        }
        s.native_map_room(&mut shapes).map_err(err)?;
        shapes
            .insert(
                s.native_text(object.id.as_str())?,
                Shape { groups: indices },
            )
            .map_err(err)?;
    }
    let witness_array = field_ref(s, state, "witnesses")?;
    #[cfg(test)]
    super::super::state::trace_event("source_witnesses", None);
    let mut seen = bj::new_map(allocation.as_ref());
    for witness in elements(s, &witness_array)? {
        super::super::history::check_deadline(deadline)?;
        exact(&witness, Kind::Object, 9)?;
        let fields = SourceFields::read_subset(
            s,
            witness.view(),
            &[
                "witness_id",
                "object_id",
                "slot",
                "merged_source_index",
                "merged_high",
                "merged_low",
                "merged_open",
                "merged_close",
                "raw_bars",
            ],
        )
        .map_err(err)?;
        let id = fields.text(s, "witness_id").map_err(err)?;
        let object_id = fields.text(s, "object_id").map_err(err)?;
        let slot = fields.integer(s, "slot").map_err(err)?;
        s.native_map_room(&mut seen).map_err(err)?;
        if !(0..3).contains(&slot)
            || seen
                .insert((s.native_text(object_id.as_str())?, slot), ())
                .map_err(err)?
                .is_some()
        {
            return Err(err("见证槽位重复/越界"));
        }
        let shape = shapes
            .get(object_id.as_str())
            .ok_or_else(|| err("见证没有同cut形态对象"))?;
        let mi = shape.groups[slot as usize];
        let group = groups.get(&mi).expect("shape was bound to existing groups");
        if fields.integer(s, "merged_source_index").map_err(err)? != group.anchor {
            return Err(err("见证锚与同cut组不同"));
        }
        let payload_fields = local_fields(s, &group.payload, &["high", "low", "open", "close"])?;
        for (merged, field_name) in [
            ("merged_high", "high"),
            ("merged_low", "low"),
            ("merged_open", "open"),
            ("merged_close", "close"),
        ] {
            let actual = fields.get_admitted(s, merged).map_err(err)?;
            let expected = if let Some(fields) = &payload_fields {
                fields.get_admitted(s, field_name).map_err(err)?
            } else {
                field(s, &group.payload, field_name)?
            };
            if actual != expected {
                return Err(err("见证合并值不是同cut组事实"));
            }
        }
        let bars = fields.get_admitted(s, "raw_bars").map_err(err)?;
        let items = elements(s, &bars)?;
        if items.len() != group.members.len() {
            return Err(err("见证raw_bars成员数不同"));
        }
        for (bar, coord) in items.iter().zip(&group.members) {
            super::super::history::check_deadline(deadline)?;
            if digest(bar)? != raw.get(coord).expect("group members were bound to raw").raw {
                return Err(err("见证raw_bars不是同cut真实原始成员"));
            }
        }
        let bars = built(bars.view())?;
        match native_identity.as_deref_mut() {
            Some(memo) => {
                memo.check_identity(s, &id, &object_id, slot as usize, mi as usize, &bars)?
            }
            None => {
                let want = super::super::generation::witness_identity(
                    s,
                    &object_id,
                    slot as usize,
                    mi as usize,
                    &bars,
                )?;
                if id != want {
                    return Err(err("见证ID不符"));
                }
            }
        }
    }
    if seen.len() != shapes.len() * 3 {
        return Err(err("形态缺完整三槽见证"));
    }
    let observations = field_ref(s, state, "observations")?;
    #[cfg(test)]
    super::super::state::trace_event("source_observations", None);
    let mut insufficient = 0;
    let mut outside_domain = 0;
    for item in elements(s, &observations)? {
        super::super::history::check_deadline(deadline)?;
        match text(s, &item, "kind")?.as_str() {
            "insufficient_knowledge" => insufficient += 1,
            "domain_not_satisfied" => outside_domain += 1,
            _ => {}
        }
    }
    Ok(Counts {
        objects: objects.len(),
        groups: groups.len(),
        windows: groups.len().saturating_sub(2),
        effective: raw.len(),
        insufficient,
        outside_domain,
    })
}
