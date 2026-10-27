//! #1394：同一次冷审计的 history → source 纯字段交接；不保存业务通过结果。
use super::super::audit_store::{CodecRead, NativeAdmission, RefFields};
use newchan_rust::s_content::capture_allocation::{
    audit_json::BusinessMap,
    native_owned::{NativeRefFields, NativeValue},
    scratch::WorkScope,
};
use newchan_rust::s_content::{self as sc, Kind, NodeDigest};

#[derive(Debug, PartialEq, Eq, Hash)]
struct RecordKey {
    format: NativeValue<String>,
    root: NativeValue<String>,
    kind: Kind,
    extent: Option<u64>,
    digest: NodeDigest,
}

#[cfg(test)]
mod tests {
    use super::super::super::audit_store::CodecRead;
    use super::super::HashReader;
    use super::*;

    #[cfg(feature = "s_incremental")]
    #[test]
    fn record_sidecar_shared_scope_seed_subset_matches_plain() {
        let mut nodes = sc::MemoryNodes::default();
        let original = sc::write_value(
            &mut nodes,
            &sc::Value::Object(vec![
                ("a".into(), sc::Value::String("original".into())),
                ("b".into(), sc::Value::Bool(true)),
            ]),
        )
        .unwrap()
        .content_ref();
        let capture = sc::AuditCapture::new(nodes);
        let mut plain_reader = HashReader::captured(&capture, None);
        let plain_fields = plain_reader
            .lookup_fields_ref((&original).into(), &["a", "b"])
            .unwrap();
        let mut plain = NativeRecordFields::new(&capture).unwrap();
        plain
            .seed((&original).into(), &["a", "b"], &plain_fields)
            .unwrap();
        let requests = [&["a"][..], &["b", "a"][..], &[][..]];
        let expected =
            requests.map(|names| plain.subset((&original).into(), names).unwrap().unwrap());
        let budget =
            super::super::super::audit_owner::native::Budget::new(8 * 1024 * 1024, 0).unwrap();
        budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let mut reader = HashReader::captured_for_audit(&capture, None)
                    .unwrap()
                    .with_codec_work(scope, &budget);
                let fields = reader.lookup_fields_ref((&original).into(), &["a", "b"])?;
                let mut records = NativeRecordFields::new_admitted(&capture, Some(scope)).unwrap();
                assert!(records.belongs_to(&capture));
                let before = budget.live();
                for _ in 0..2 {
                    let attempts = scope.snapshot().attempts;
                    records.seed((&original).into(), &["a", "b"], &fields)?;
                    assert!(scope.snapshot().attempts > attempts);
                }
                for (names, expected) in requests.into_iter().zip(&expected) {
                    let actual = records.subset((&original).into(), names)?.unwrap();
                    assert!(actual.iter().eq(expected.iter()));
                }
                assert!(budget.live() > before);
                let retained = budget.live();
                records.clear();
                assert!(budget.live() < retained, "clear 释放实际条目");
                assert!(budget.live() > before, "clear 仍持有实际 hash buffer");
                drop(records);
                assert_eq!(budget.live(), before);
                let records = NativeRecordFields::new_admitted(&capture, Some(scope)).unwrap();
                assert!(records.subset((&original).into(), &["a"])?.is_none());
                Ok::<_, sc::CodecError>(())
            })
            .unwrap();
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn record_sidecar_subset_outlives_scope_and_drops_its_own_permits() {
        let mut nodes = sc::MemoryNodes::default();
        let original = sc::write_value(
            &mut nodes,
            &sc::Value::Object(vec![("a".into(), sc::Value::String("value".into()))]),
        )
        .unwrap()
        .content_ref();
        let capture = sc::AuditCapture::new(nodes);
        let mut reader = HashReader::captured(&capture, None);
        let fields = reader
            .lookup_fields_ref((&original).into(), &["a"])
            .unwrap();
        let budget = super::super::super::audit_owner::native::Budget::new(1 << 20, 0).unwrap();
        let baseline = budget.live();
        for _ in 0..3 {
            let subset = budget
                .codec_work()
                .unwrap()
                .run(|scope| {
                    let mut records =
                        NativeRecordFields::new_admitted(&capture, Some(scope)).unwrap();
                    records.seed((&original).into(), &["a"], &fields)?;
                    let before = budget.live();
                    let subset = records.subset((&original).into(), &["a"])?.unwrap();
                    assert!(budget.live() > before, "subset 和原字段独立持费");
                    records.clear();
                    assert_eq!(
                        subset.get("a").unwrap().root.as_str(),
                        fields.get("a").unwrap().root()
                    );
                    Ok::<_, sc::CodecError>(subset)
                })
                .unwrap();
            assert!(budget.live() > baseline, "离域结果仍持原许可");
            assert_eq!(
                subset.get("a").unwrap().root.as_str(),
                fields.get("a").unwrap().root()
            );
            drop(subset);
            assert_eq!(budget.live(), baseline, "所有结果销毁后只剩父账本元数据");
        }
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn record_sidecar_failed_replacement_keeps_original_and_releases_temporaries() {
        let mut nodes = sc::MemoryNodes::default();
        let original = sc::write_value(
            &mut nodes,
            &sc::Value::Object(vec![("a".into(), sc::Value::String("value".into()))]),
        )
        .unwrap()
        .content_ref();
        let capture = sc::AuditCapture::new(nodes);
        let mut reader = HashReader::captured(&capture, None);
        let fields = reader
            .lookup_fields_ref((&original).into(), &["a"])
            .unwrap();
        let limit = 1 << 20;
        let budget = super::super::super::audit_owner::native::Budget::new(limit, 0).unwrap();
        let baseline = budget.live();
        let result = budget.codec_work().unwrap().run(|scope| {
            let mut records = NativeRecordFields::new_admitted(&capture, Some(scope)).unwrap();
            records.seed((&original).into(), &["a"], &fields)?;
            let before = budget.live();
            let occupied = budget.reserve(limit - before).unwrap();
            let error = records
                .seed((&original).into(), &["a"], &fields)
                .unwrap_err();
            // 失败后仅借用观察旧值，绝不在失败工作域重试另一条准入路径。
            assert_eq!(records.records.len(), 1);
            let (key, saved) = records.records.iter().next().unwrap();
            assert_eq!(key.format.as_str(), original.format.as_str());
            assert_eq!(key.root.as_str(), original.root.as_str());
            assert_eq!(key.kind, original.kind);
            assert_eq!(key.extent, original.extent);
            assert_eq!(
                saved.get("a").unwrap().root.as_str(),
                fields.get("a").unwrap().root()
            );
            drop(occupied);
            assert_eq!(budget.live(), before, "失败构造不留临时收费");
            Err::<(), _>(error)
        });
        assert!(result.is_err());
        assert_eq!(budget.live(), baseline);
    }

    #[test]
    fn record_sidecar_full_ref_capture_and_clear_isolation() {
        let mut nodes = sc::MemoryNodes::default();
        let original = sc::write_value(
            &mut nodes,
            &sc::Value::Object(vec![
                ("a".into(), sc::Value::String("original".into())),
                ("b".into(), sc::Value::Bool(true)),
            ]),
        )
        .unwrap()
        .content_ref();
        let another = sc::AuditCapture::new(nodes.clone());
        let capture = sc::AuditCapture::new(nodes);
        let mut reader = HashReader::captured(&capture, None);
        let fields = reader
            .lookup_fields_ref((&original).into(), &["a", "b"])
            .unwrap();
        let mut records = NativeRecordFields::new(&capture).unwrap();
        assert!(records.belongs_to(&capture));
        assert!(
            !records.belongs_to(&another),
            "相同原件集合不是同一capture owner"
        );
        records
            .seed((&original).into(), &["a", "b"], &fields)
            .unwrap();
        for names in [&["a"][..], &["b", "a"][..]] {
            let got = records.subset((&original).into(), names).unwrap().unwrap();
            for name in names {
                let expected = fields.get(name).unwrap();
                let actual: sc::RefView<'_> = got.get(name).unwrap().into();
                assert_eq!(actual.format(), expected.format());
                assert_eq!(actual.root(), expected.root());
                assert_eq!(actual.kind(), expected.kind());
                assert_eq!(actual.extent(), expected.extent());
            }
        }
        assert!(records
            .subset((&original).into(), &["a", "a"])
            .unwrap()
            .is_none());
        assert!(records
            .subset((&original).into(), &["missing"])
            .unwrap()
            .is_none());
        for changed in 0..4 {
            let mut forged = original.clone();
            match changed {
                0 => forged.format = "bad-format".into(),
                1 => forged.kind = Kind::Array,
                2 => forged.extent = Some(3),
                _ => forged.root.push('0'),
            }
            assert!(!matches!(
                records.subset((&forged).into(), &["a"]),
                Ok(Some(_))
            ));
            assert!(records
                .seed((&forged).into(), &["a", "b"], &fields)
                .is_err());
        }
        let overlay = sc::write_value(
            &mut reader,
            &sc::Value::Object(vec![("a".into(), sc::Value::String("overlay".into()))]),
        )
        .unwrap()
        .content_ref();
        let overlay_fields = reader.lookup_fields_ref((&overlay).into(), &["a"]).unwrap();
        records
            .seed((&overlay).into(), &["a"], &overlay_fields)
            .unwrap();
        assert!(records.subset((&overlay).into(), &["a"]).unwrap().is_none());
        reader.clear_overlay();
        // 只保存capture原件；清除overlay不会把其临时内容变成可复用材料。
        assert!(records.subset((&overlay).into(), &["a"]).unwrap().is_none());
        assert!(records
            .subset((&original).into(), &["a"])
            .unwrap()
            .is_some());
        records.clear();
        assert!(records
            .subset((&original).into(), &["a"])
            .unwrap()
            .is_none());
        assert!(NativeRecordFields::new(&another)
            .unwrap()
            .subset((&original).into(), &["a"])
            .unwrap()
            .is_none());
    }
}
impl RecordKey {
    fn new(r: sc::RefView<'_>, admission: &NativeAdmission<'_>) -> Result<Self, sc::CodecError> {
        Ok(Self {
            format: admission.native_string(r.format())?,
            root: admission.native_string(r.root())?,
            kind: r.kind(),
            extent: r.extent(),
            digest: r.digest()?,
        })
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NativeRecordFieldStats {
    pub full_record_seeds: u64,
    pub source_subset_hits: u64,
}
#[cfg(test)]
thread_local! {
    static STATS: std::cell::Cell<NativeRecordFieldStats> = const { std::cell::Cell::new(NativeRecordFieldStats { full_record_seeds: 0, source_subset_hits: 0 }) };
}
#[cfg(test)]
#[cfg_attr(
    not(feature = "s_incremental"),
    allow(
        dead_code,
        reason = "#1608：测试辅助，仅 s_incremental 测试使用"
    )
)]
pub(crate) fn reset_native_record_field_stats() {
    STATS.with(|s| s.set(NativeRecordFieldStats::default()));
}
#[cfg(test)]
#[cfg_attr(
    not(feature = "s_incremental"),
    allow(
        dead_code,
        reason = "#1608：测试辅助，仅 s_incremental 测试使用"
    )
)]
pub(crate) fn native_record_field_stats() -> NativeRecordFieldStats {
    STATS.with(std::cell::Cell::get)
}

/// 独立 Native sidecar；普通/Owned 的集合及计费布局均不携带此状态。
pub(crate) struct NativeRecordFields<'capture> {
    capture: &'capture sc::AuditCapture,
    work: Option<&'capture WorkScope>,
    records: BusinessMap<RecordKey, NativeRefFields>,
}
impl<'capture> NativeRecordFields<'capture> {
    #[cfg(test)]
    pub(crate) fn new(capture: &'capture sc::AuditCapture) -> Option<Self> {
        Self::new_admitted(capture, None)
    }
    #[cfg_attr(
        all(not(test), not(feature = "s_incremental")),
        allow(
            dead_code,
            reason = "#1608：默认 feature 的非测试构建不触达；测试与 s_incremental 构建使用"
        )
    )]
    pub(crate) fn new_admitted(
        capture: &'capture sc::AuditCapture,
        work: Option<&'capture WorkScope>,
    ) -> Option<Self> {
        (!capture.is_allocation_prototype()).then(|| Self {
            capture,
            work,
            records: BusinessMap::plain(),
        })
    }
    pub(crate) fn belongs_to(&self, capture: &sc::AuditCapture) -> bool {
        std::ptr::eq(self.capture, capture)
    }
    /// 原无计量包装继续供既有测试使用。
    #[cfg(test)]
    pub(crate) fn seed(
        &mut self,
        r: sc::RefView<'_>,
        names: &[&str],
        fields: &RefFields<'_>,
    ) -> Result<(), sc::CodecError> {
        self.seed_measured(r, names, fields, None)
    }
    pub(crate) fn seed_measured(
        &mut self,
        r: sc::RefView<'_>,
        names: &[&str],
        fields: &RefFields<'_>,
        measure: Option<&super::super::audit_budget::NativeMeasure<'_>>,
    ) -> Result<(), sc::CodecError> {
        let sc::RefView::Normal(original) = r else {
            return Ok(());
        };
        if !self.capture.contains(&original.digest()?) {
            return Ok(());
        }
        let admission = NativeAdmission(self.work);
        admission.native_verify_original_measured(self.capture, r, sc::Mode::Public, measure)?;
        // 不从 Owned 结果或不完整字段集补造原件。
        let RefFields::Plain(fields) = fields else {
            return Ok(());
        };
        if r.kind() != Kind::Object
            || r.extent() != Some(names.len() as u64)
            || fields.len() != names.len()
            || names.iter().any(|name| fields.get(name).is_none())
        {
            return Ok(());
        }
        let originals = NativeRefFields::try_from_views(
            names
                .iter()
                .enumerate()
                .filter(|(at, name)| !names[..*at].contains(name))
                .map(|(_, name)| (*name, fields.get(name).unwrap().into())),
            self.work,
        )?;
        let key = RecordKey::new(r, &admission)?;
        if !self.records.contains_key(&key) {
            admission.native_map_room(&mut self.records)?;
        }
        // 全部新载荷准入成功后才替换；旧字段的原许可随旧 owner 析构归还。
        self.records.insert(key, originals)?;
        #[cfg(test)]
        STATS.with(|s| {
            let mut stats = s.get();
            stats.full_record_seeds += 1;
            s.set(stats);
        });
        Ok(())
    }
    pub(crate) fn subset(
        &self,
        r: sc::RefView<'_>,
        names: &[&str],
    ) -> Result<Option<NativeRefFields>, sc::CodecError> {
        if !matches!(r, sc::RefView::Normal(_)) {
            return Ok(None);
        }
        // subset 的原核同样拒绝重复 requested names；命中也不能跳过该门。
        if names
            .iter()
            .enumerate()
            .any(|(at, name)| names[..at].contains(name))
        {
            return Ok(None);
        }
        let admission = NativeAdmission(self.work);
        let key = RecordKey::new(r, &admission)?;
        let Some(fields) = self.records.get(&key) else {
            return Ok(None);
        };
        // 缺字段沿原 lookup 路径报错；不将一个部分结果冒充成功。
        if names.iter().any(|name| fields.get(name).is_none()) {
            return Ok(None);
        }
        let subset = NativeRefFields::try_from_views(
            names
                .iter()
                .map(|name| (*name, fields.get(name).unwrap().into())),
            self.work,
        )?;
        #[cfg(test)]
        STATS.with(|s| {
            let mut stats = s.get();
            stats.source_subset_hits += 1;
            s.set(stats);
        });
        Ok(Some(subset))
    }
    pub(crate) fn clear(&mut self) {
        // 清除实际条目会析构键及字段 owner；hash buffer 仍存在，保留其原许可。
        self.records.retain(|_, _| false);
    }
}
