use super::*;
use sc::capture_allocation::audit_json::{JsonArray, JsonView};

pub(crate) struct ObjectRow {
    pub(crate) id: NativeValue<String>,
    pub(crate) record: NodeDigest,
    pub(crate) sources: NodeDigest,
    pub(crate) first_generation: i64,
    pub(crate) first_cut: NativeValue<String>,
    pub(crate) published_generation: i64,
}
pub(crate) struct RelatedRow {
    pub(crate) id: NativeValue<String>,
    pub(crate) record: NodeDigest,
    pub(crate) sources: NodeDigest,
}
pub(crate) struct BusinessRows {
    pub(crate) objects: sc::CodecVec<ObjectRow>,
    pub(crate) withdrawals: sc::CodecVec<NativeValue<String>>,
    pub(crate) witnesses: sc::CodecVec<RelatedRow>,
    pub(crate) observations: sc::CodecVec<RelatedRow>,
}

/// 36M cover 行：键是本 cut 已核生效的 source_coord；owner 为同 cut CC005 的 object_id，None 即已发布未定原件 Pending。
pub(crate) struct CoverRow {
    pub(crate) coord: i64,
    pub(crate) seq: i64,
    pub(crate) owner: Option<NativeValue<String>>,
}
/// 同一次来源核导出的事实投影；只在分区核成功后签发，不能由调用方拼装。
/// `removed` 是本代失去生效资格的坐标（修订改坐标）；`effective` 是本 cut 生效坐标总数。
#[derive(Default)]
pub(crate) struct SourceExports {
    pub(crate) observation_sources: sc::CodecVec<(NodeDigest, NodeDigest)>,
    pub(crate) cover: sc::CodecVec<CoverRow>,
    pub(crate) removed: sc::CodecVec<i64>,
    pub(crate) effective: usize,
}
impl SourceExports {
    pub(crate) fn push_cover(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        coord: i64,
        seq: i64,
        owner: Option<&str>,
    ) -> Result<(), String> {
        let owner = owner
            .map(|id| s.native_string(id))
            .transpose()
            .map_err(err)?;
        s.native_codec_room(&mut self.cover, 1).map_err(err)?;
        self.cover
            .try_push(CoverRow { coord, seq, owner })
            .map_err(err)
    }
    pub(crate) fn push_removed(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        coord: i64,
    ) -> Result<(), String> {
        s.native_codec_room(&mut self.removed, 1).map_err(err)?;
        self.removed.try_push(coord).map_err(err)
    }
    pub(crate) fn finish(&mut self) {
        self.observation_sources.sort_unstable_by_key(|row| row.0);
        self.cover.sort_unstable_by_key(|row| row.coord);
        self.removed.sort_unstable();
    }
}

pub(crate) fn coordinates_digest(
    s: &(impl CodecRead + ?Sized),
    values: &[i64],
) -> Result<NodeDigest, String> {
    struct Coordinates(sc::CodecVec<bj::Decimal>);
    impl JsonArray for Coordinates {
        fn len(&self) -> usize {
            self.0.len()
        }
        fn get(&self, i: usize) -> Option<JsonView<'_>> {
            self.0.get(i).map(|n| JsonView::String(n.text()))
        }
    }
    let mut coordinates = Coordinates(sc::CodecVec::default());
    s.native_codec_room(&mut coordinates.0, values.len())
        .map_err(err)?;
    for &value in values {
        coordinates
            .0
            .try_push(bj::Decimal::new(value))
            .map_err(err)?;
    }
    super::super::super::history::hash_json_in_store(
        s,
        JsonView::BorrowedArray(&coordinates),
        None,
        None,
    )
}

pub(crate) fn observation_sources<'a>(
    s: &mut (impl CodecRead + ?Sized),
    record: &RefValue,
    mut group_sources: impl FnMut(i64) -> Option<&'a [i64]>,
) -> Result<NodeDigest, String> {
    let detail = field(s, record, "detail")?;
    let coordinates = match s
        .lookup_optional_field_ref(detail.view(), "source_coords")
        .map_err(err)?
    {
        Some(source) => coords(s, &source)?,
        None => {
            let mut values = sc::CodecVec::default();
            for name in ["window_start", "window_mid", "window_end"] {
                let window = scalar(s, record, name)?;
                if window.view().is_null() {
                    continue;
                }
                let index = window
                    .view()
                    .as_i64()
                    .ok_or_else(|| err("P6观察窗口不是组位置"))?;
                let sources = group_sources(index).ok_or_else(|| err("P6观察窗口缺同cut组来源"))?;
                values = union_coords(s, &values, sources)?;
            }
            values
        }
    };
    coordinates_digest(s, &coordinates)
}

fn object_record(
    s: &mut (impl CodecStore + ?Sized),
    root: sc::RefView<'_>,
) -> Result<NodeDigest, String> {
    let original = bj::render_json(s, root)?;
    let fields = original
        .view()
        .as_object()
        .ok_or_else(|| err("P6对象原件不是对象"))?;
    let mut unstamped = sc::CodecVec::default();
    s.native_codec_room(&mut unstamped, fields.len())
        .map_err(err)?;
    for (name, value) in fields.iter() {
        if !matches!(
            name,
            "first_known_generation" | "first_known_cut" | "published_generation"
        ) {
            unstamped.try_push((name, value)).map_err(err)?;
        }
    }
    let record = JsonView::object_fields(&unstamped).map_err(err)?;
    super::super::super::history::hash_json_in_store(s, record, None, None)
}
pub(crate) fn export(
    s: &mut (impl CodecStore + ?Sized),
    objects: &super::super::super::object_index::Plan,
    auxiliary: &super::super::super::aux_index::Plan,
    observation_sources: &[(NodeDigest, NodeDigest)],
) -> Result<BusinessRows, String> {
    export_parts(
        s,
        objects.births.iter(),
        &objects.withdrawals,
        &objects.replaces,
        auxiliary.births.witnesses.iter(),
        auxiliary.births.observations.iter(),
        observation_sources,
    )
}

/// #1481：冷折叠只投影已由历史核验过的独立索引，出生与撤回字段导出复用在线同核。
#[cfg(feature = "s_incremental")]
pub(crate) fn export_history(
    s: &mut (impl CodecStore + ?Sized),
    objects: &[super::super::super::object_index::Entry],
    auxiliary: &super::super::super::aux_index::Index,
    generation: i64,
    observation_sources: &[(NodeDigest, NodeDigest)],
) -> Result<BusinessRows, String> {
    let (withdrawals, replaces) =
        super::super::super::object_index::changes_at_admitted(s, objects, generation, None)?;
    export_parts(
        s,
        objects
            .iter()
            .filter(|row| row.life.published_generation == generation),
        &withdrawals,
        &replaces,
        auxiliary
            .witnesses
            .iter()
            .filter(|row| row.first == generation),
        auxiliary
            .observations
            .iter()
            .filter(|row| row.first == generation),
        observation_sources,
    )
}

fn export_parts<'a>(
    s: &mut (impl CodecStore + ?Sized),
    objects: impl Iterator<Item = &'a super::super::super::object_index::Entry>,
    withdrawals: &bj::JsonOwner,
    replaces: &bj::JsonOwner,
    witnesses: impl Iterator<Item = &'a super::super::super::aux_index::Witness>,
    observations: impl Iterator<Item = &'a super::super::super::aux_index::Observation>,
    observation_sources: &[(NodeDigest, NodeDigest)],
) -> Result<BusinessRows, String> {
    let mut rows = BusinessRows {
        objects: sc::CodecVec::default(),
        withdrawals: sc::CodecVec::default(),
        witnesses: sc::CodecVec::default(),
        observations: sc::CodecVec::default(),
    };
    #[cfg(all(test, feature = "s_incremental"))]
    let mut _test_budget_reservation = None;
    for entry in objects {
        let reference = s.native_built_ref(&entry.generated).map_err(err)?;
        let root = reference.view();
        let record = object_record(s, root)?;
        let source_text = field(s, root, "source_coords_json")?;
        let sources = s
            .lookup_ref(source_text.view(), &[PathStep::JsonText])
            .map_err(err)?;
        let first_generation = number(s, root, "first_known_generation")?;
        let first_cut = text(s, root, "first_known_cut")?;
        let published_generation = number(s, root, "published_generation")?;
        s.native_codec_room(&mut rows.objects, 1).map_err(err)?;
        rows.objects
            .try_push(ObjectRow {
                id: s.native_string(entry.id.as_str()).map_err(err)?,
                record,
                sources: digest(&sources)?,
                first_generation,
                first_cut: s.native_string(first_cut.as_str()).map_err(err)?,
                published_generation,
            })
            .map_err(err)?;
        #[cfg(all(test, feature = "s_incremental"))]
        if _test_budget_reservation.is_none() {
            _test_budget_reservation = fee_probe::block_after_object();
        }
    }
    for withdrawal in bj::RawEventsView::from(withdrawals) {
        let id = bj::field(withdrawal, "object_id")
            .as_str()
            .ok_or_else(|| err("P6撤回缺身份"))?;
        s.native_codec_room(&mut rows.withdrawals, 1).map_err(err)?;
        rows.withdrawals
            .try_push(s.native_string(id).map_err(err)?)
            .map_err(err)?;
    }
    rows.objects
        .sort_unstable_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
    rows.withdrawals
        .sort_unstable_by(|a, b| a.as_str().cmp(b.as_str()));
    for replacement in bj::RawEventsView::from(replaces) {
        let old = bj::field(replacement, "old_object_id")
            .as_str()
            .ok_or_else(|| err("P6替换缺旧身份"))?;
        let new = bj::field(replacement, "new_object_id")
            .as_str()
            .ok_or_else(|| err("P6替换缺新身份"))?;
        if rows
            .withdrawals
            .binary_search_by(|id| id.as_str().cmp(old))
            .is_err()
            || rows
                .objects
                .binary_search_by(|row| row.id.as_str().cmp(new))
                .is_err()
        {
            return Err(err("P6替换未对应本代撤回与出生"));
        }
    }
    for entry in witnesses {
        let bars = field(s, entry.generated.view(), "raw_bars")?;
        let mut coordinates = sc::CodecVec::default();
        for bar in elements(s, &bars)? {
            let value = text(s, bar.view(), "source_coord")?;
            let coordinate = crate::parse_source_coord(value.as_str()).map_err(err)?;
            s.native_codec_room(&mut coordinates, 1).map_err(err)?;
            coordinates.try_push(coordinate).map_err(err)?;
        }
        coordinates.sort_unstable();
        if coordinates.windows(2).any(|v| v[0] == v[1]) {
            return Err(err("P6见证来源身份重复"));
        }
        let sources = coordinates_digest(s, &coordinates)?;
        s.native_codec_room(&mut rows.witnesses, 1).map_err(err)?;
        rows.witnesses
            .try_push(RelatedRow {
                id: s.native_string(entry.id.as_str()).map_err(err)?,
                record: digest(entry.generated.view())?,
                sources,
            })
            .map_err(err)?;
    }
    for entry in observations {
        let record = digest(entry.generated.view())?;
        let index = observation_sources
            .binary_search_by_key(&record, |row| row.0)
            .map_err(|_| err("P6观察出生缺同次来源投影"))?;
        s.native_codec_room(&mut rows.observations, 1)
            .map_err(err)?;
        rows.observations
            .try_push(RelatedRow {
                id: s.native_string(entry.id.as_str()).map_err(err)?,
                record,
                sources: observation_sources[index].1,
            })
            .map_err(err)?;
    }
    Ok(rows)
}
