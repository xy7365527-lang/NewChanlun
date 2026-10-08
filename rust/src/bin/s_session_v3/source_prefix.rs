//! #1394：当前 Native Capture 内的来源纯解析及有效前缀证书。
//! 不取得 closure 资格；调用者每 cut 仍先核全部当前原件。
use super::*;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

#[cfg(test)]
thread_local! {
    static STATS: std::cell::Cell<Option<[u64; 4]>> = const { std::cell::Cell::new(None) };
}
#[cfg(test)]
pub(super) fn reset_stats() {
    STATS.with(|s| s.set(None));
}
#[cfg(test)]
pub(super) fn stats() -> Option<[u64; 4]> {
    STATS.with(|s| s.get())
}

#[derive(Debug)]
pub(super) enum SourceCoords {
    Original(sc::CodecVec<i64>),
    Shared(Rc<[i64]>),
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod tests {
    use super::*;
    fn event(identity: &str, coord: &str, revision: i64, seq: i64) -> Value {
        json!({"identity_key":identity,"source_coord":coord,"revision":revision,"seq":seq,
            "payload_hash":"same-price", "receipt_id":format!("r-{seq}"),
            "event_id":format!("e-{seq}"),"input_revision":1})
    }
    fn ref_value(event: &Value, numeric: bool) -> Value {
        serde_json::to_value(bj::RawReference::new(event.into(), numeric).unwrap()).unwrap()
    }
    fn raw_map(raw: &[Value], n: usize) -> BusinessMap<i64, Source> {
        let effective = bj::effective(RawEventsView::plain(&raw[..n]), None).unwrap();
        let mut map = bj::new_map(None);
        for event in effective.iter() {
            let wire = bj::RawReference::new(event, false).unwrap();
            let numeric = bj::RawReference::new(event, true).unwrap();
            map.insert(
                bj::coord(event).unwrap(),
                Source {
                    wire: sc::write_json_view(&mut HashOnly, wire.view())
                        .unwrap()
                        .digest,
                    numeric: sc::write_json_view(&mut HashOnly, numeric.view())
                        .unwrap()
                        .digest,
                    raw: sc::write_json_view(&mut HashOnly, event).unwrap().digest,
                    bar: source_pending::raw_bar(&sc::MemoryNodes::default(), event, None, None)
                        .unwrap(),
                },
            )
            .unwrap();
        }
        map
    }
    fn build(nodes: &mut sc::MemoryNodes, value: &Value) -> RefValue {
        RefValue::Plain(
            sc::write_value(nodes, &cv(value).unwrap())
                .unwrap()
                .content_ref(),
        )
    }
    fn read<'a>(capture: &'a sc::AuditCapture) -> super::super::super::history::HashReader<'a> {
        super::super::super::history::HashReader::captured(capture, None)
    }
    #[test]
    fn native_prefix_certificates_and_shared_coords_use_current_parent_scope() {
        let raw = vec![
            event("a", "10", 1, 1),
            event("a", "10", 2, 2),
            event("b", "30", 1, 3),
        ];
        let expected = NativeSourceMemo::winner_intervals((&raw).into()).unwrap();
        let capture = sc::AuditCapture::new(sc::MemoryNodes::default());
        let mut memo = NativeSourceMemo::new(&capture, (&raw).into());
        assert!(!memo.intervals_ready && memo.intervals.is_none());
        let budget = super::super::super::audit_owner::native::Budget::new(1024 * 1024, 0).unwrap();
        let before = budget.live();
        budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let admission = super::super::super::audit_store::NativeAdmission(Some(scope));
                let intervals =
                    NativeSourceMemo::winner_intervals_admitted(&admission, (&raw).into())?
                        .unwrap();
                assert_eq!(intervals, expected);
                let shared = memo.share(&admission, copy_coords(&admission, &[10, 30])?)?;
                let again = memo.share(&admission, copy_coords(&admission, &[10, 30])?)?;
                assert!(Rc::ptr_eq(&shared, &again));
                assert_eq!(&*shared, &[10, 30]);
                assert!(
                    budget.live()
                        > before + sc::capture_allocation::scratch::CodecWork::metadata_bytes()
                );
                drop(shared);
                drop(again);
                drop(intervals);
                memo.clear();
                assert!(
                    budget.live()
                        > before + sc::capture_allocation::scratch::CodecWork::metadata_bytes(),
                    "容器析构后保守许可仍归候选父Budget"
                );
                drop(memo);
                Ok::<_, String>(())
            })
            .unwrap();
        assert!(budget.live() > before, "WorkScope结束不归还Native保守持账");
        assert_eq!(budget.scratch_observation().0, 0);
        assert_eq!(budget.scratch_observation().2, 0);
    }

    #[test]
    fn winner_sweep_matches_every_effective_prefix_with_revisions_and_sparse_coords() {
        // Acceptance ordinal and source coordinate deliberately have different order/spacing.
        let raw = vec![
            event("a", "900", 3, 40),
            event("b", "7", 1, 20),
            event("a", "900", 3, 1),
            event("a", "900", 2, 80),
            event("b", "7", 5, 99),
            event("a", "900", 4, 2),
        ];
        let intervals = NativeSourceMemo::winner_intervals((&raw).into()).unwrap();
        for n in 0..=raw.len() {
            let effective = raw_map(&raw, n);
            let active = intervals
                .iter()
                .filter(|(_, runs)| runs.iter().any(|run| run.contains(n)))
                .map(|(key, _)| *key)
                .collect::<std::collections::HashSet<_>>();
            let wanted = effective
                .iter()
                .flat_map(|(coord, source)| {
                    [(*coord, source.wire, false), (*coord, source.numeric, true)]
                })
                .collect::<std::collections::HashSet<_>>();
            assert_eq!(active, wanted, "prefix={n}");
        }
    }
    #[test]
    fn references_hit_shared_coords_then_reject_same_price_revision_and_future_sources() {
        let raw = vec![
            event("a", "100", 1, 0),
            event("b", "900", 1, 1),
            event("a", "100", 2, 2),
        ];
        let mut nodes = sc::MemoryNodes::default();
        let refs = build(&mut nodes, &json!([ref_value(&raw[0], false)]));
        let coords = build(&mut nodes, &json!(["100"]));
        let future = build(&mut nodes, &json!([ref_value(&raw[1], false)]));
        let numeric = build(&mut nodes, &json!([ref_value(&raw[0], true)]));
        let capture = sc::AuditCapture::new(nodes);
        let mut memo = NativeSourceMemo::new(&capture, (&raw).into());
        let mut reader = read(&capture);
        let mut s = super::super::super::audit_store::StoreView::for_audit(&mut reader);
        let parsed = memo
            .coords(&mut s, &coords, ParseDomain::JsonInner)
            .unwrap();
        for n in [1, 2, 1, 2] {
            let values = memo
                .references(
                    &mut s,
                    &refs,
                    &raw_map(&raw, n),
                    false,
                    &mut bj::new_map(None),
                    n,
                    ParseDomain::JsonInner,
                )
                .unwrap();
            assert_eq!(values, parsed);
            let (SourceCoords::Shared(a), SourceCoords::Shared(b)) = (&values, &parsed) else {
                panic!("not shared")
            };
            assert!(Rc::ptr_eq(a, b));
        }
        assert_eq!(memo.stats[2], 3);
        assert!(memo
            .references(
                &mut s,
                &refs,
                &raw_map(&raw, 3),
                false,
                &mut bj::new_map(None),
                3,
                ParseDomain::JsonInner
            )
            .unwrap_err()
            .contains("完整八字段"));
        assert!(memo
            .references(
                &mut s,
                &future,
                &raw_map(&raw, 1),
                false,
                &mut bj::new_map(None),
                1,
                ParseDomain::JsonInner
            )
            .unwrap_err()
            .contains("生效来源"));
        assert!(memo
            .references(
                &mut s,
                &refs,
                &raw_map(&raw, 1),
                true,
                &mut bj::new_map(None),
                1,
                ParseDomain::JsonInner
            )
            .is_err());
        memo.references(
            &mut s,
            &numeric,
            &raw_map(&raw, 1),
            true,
            &mut bj::new_map(None),
            1,
            ParseDomain::JsonInner,
        )
        .unwrap();
        assert!(memo
            .references(
                &mut s,
                &numeric,
                &raw_map(&raw, 1),
                false,
                &mut bj::new_map(None),
                1,
                ParseDomain::JsonInner
            )
            .is_err());
        assert!(memo
            .references
            .get(&SequenceKey::new(&refs, ParseDomain::Public, false).unwrap())
            .is_none());
        memo.references(
            &mut s,
            &refs,
            &raw_map(&raw, 1),
            false,
            &mut bj::new_map(None),
            1,
            ParseDomain::Public,
        )
        .unwrap();
        assert_eq!(memo.stats[3], 7);
        let mut changed = raw.clone();
        changed[0]["receipt_id"] = json!("changed-owner");
        let mut fresh = NativeSourceMemo::new(&capture, (&changed).into());
        assert!(fresh.references.is_empty());
        assert!(fresh
            .references(
                &mut s,
                &refs,
                &raw_map(&changed, 1),
                false,
                &mut bj::new_map(None),
                1,
                ParseDomain::JsonInner
            )
            .is_err());
        let empty = sc::AuditCapture::new(sc::MemoryNodes::default());
        let fresh = NativeSourceMemo::new(&empty, (&raw).into());
        assert!(fresh.coords.is_empty() && fresh.references.is_empty());
    }
    #[test]
    fn future_invalid_or_colliding_raw_disables_only_reference_reuse() {
        for invalid in [
            event("b", "-1", 1, 1),
            event("b", "01", 1, 1),
            event("b", "100", 1, 1),
            event("a", "101", 2, 1),
        ] {
            let raw = vec![event("a", "100", 1, 0), invalid];
            let mut nodes = sc::MemoryNodes::default();
            let refs = build(&mut nodes, &json!([ref_value(&raw[0], false)]));
            let capture = sc::AuditCapture::new(nodes);
            let mut memo = NativeSourceMemo::new(&capture, (&raw).into());
            assert!(memo.intervals.is_none());
            let mut reader = read(&capture);
            let mut s = super::super::super::audit_store::StoreView::for_audit(&mut reader);
            memo.references(
                &mut s,
                &refs,
                &raw_map(&raw, 1),
                false,
                &mut bj::new_map(None),
                1,
                ParseDomain::JsonInner,
            )
            .unwrap();
            assert!(memo.references.is_empty());
            if bj::coord((&raw[1]).into()).is_err() || raw[1]["identity_key"] == "b" {
                assert!(bj::effective((&raw).into(), None).is_err());
            }
        }
    }
    #[test]
    fn cached_coords_preserve_order_lexemes_negative_and_domain_rules() {
        let mut nodes = sc::MemoryNodes::default();
        let valid = build(&mut nodes, &json!(["-10", "0", "900"]));
        let invalid = [
            json!(["1", "1"]),
            json!(["2", "1"]),
            json!(["01"]),
            json!(["-0"]),
            json!(["+1"]),
            json!(["1.0"]),
            json!([1]),
            json!(["9223372036854775808"]),
        ];
        let roots = invalid
            .iter()
            .map(|v| build(&mut nodes, v))
            .collect::<Vec<_>>();
        let capture = sc::AuditCapture::new(nodes);
        let mut memo = NativeSourceMemo::new(&capture, RawEventsView::plain(&[]));
        let mut reader = read(&capture);
        let mut s = super::super::super::audit_store::StoreView::for_audit(&mut reader);
        for domain in [ParseDomain::Public, ParseDomain::JsonInner] {
            for _ in 0..2 {
                assert_eq!(
                    &*memo.coords(&mut s, &valid, domain).unwrap(),
                    &[-10, 0, 900]
                );
            }
            for root in &roots {
                assert!(memo.coords(&mut s, root, domain).is_err());
            }
        }
        assert_eq!(memo.coords.len(), 2);
        assert_eq!(memo.stats[0], 2);
    }
}
impl std::ops::Deref for SourceCoords {
    type Target = [i64];
    fn deref(&self) -> &[i64] {
        match self {
            Self::Original(v) => v,
            Self::Shared(v) => v,
        }
    }
}
impl PartialEq for SourceCoords {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Shared(a), Self::Shared(b)) if Rc::ptr_eq(a, b) => true,
            _ => **self == **other,
        }
    }
}
impl<'a> IntoIterator for &'a SourceCoords {
    type Item = &'a i64;
    type IntoIter = std::slice::Iter<'a, i64>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum ParseDomain {
    Public,
    JsonInner,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SequenceKey {
    digest: NodeDigest,
    kind: Kind,
    extent: Option<u64>,
    domain: ParseDomain,
    numeric: bool,
}
impl SequenceKey {
    fn new(r: &RefValue, domain: ParseDomain, numeric: bool) -> Result<Self, String> {
        Ok(Self {
            digest: digest(r)?,
            kind: r.kind(),
            extent: r.view().extent(),
            domain,
            numeric,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Interval {
    first: usize,
    last: usize,
}
impl Interval {
    fn contains(self, n: usize) -> bool {
        self.first <= n && n <= self.last
    }
    fn intersect(self, other: Self) -> Self {
        Self {
            first: self.first.max(other.first),
            last: self.last.min(other.last),
        }
    }
}
struct Proof {
    values: Rc<[i64]>,
    interval: Interval,
}
pub(super) struct NativeSourceMemo<'capture> {
    capture: &'capture sc::AuditCapture,
    raw: RawEventsView<'capture>,
    raw_len: usize,
    intervals_ready: bool,
    // None means no optimization qualification; original reference predicate still runs.
    intervals: Option<HashMap<(i64, NodeDigest, bool), Vec<Interval>>>,
    coords: HashMap<SequenceKey, Rc<[i64]>>,
    references: HashMap<SequenceKey, Proof>,
    intern: HashMap<Rc<[i64]>, Rc<[i64]>>,
    #[cfg(test)]
    pub(super) stats: [u64; 4], // coords hit/miss, refs hit/miss
}
impl<'capture> NativeSourceMemo<'capture> {
    pub(super) fn new(capture: &'capture sc::AuditCapture, raw: RawEventsView<'capture>) -> Self {
        #[cfg(test)]
        STATS.with(|s| s.set(Some([0; 4])));
        Self {
            capture,
            raw,
            raw_len: raw.len(),
            // restore 先接父 scope，再在真实消费时建立证书，构造失败不降级放行。
            intervals_ready: false,
            intervals: None,
            coords: HashMap::new(),
            references: HashMap::new(),
            intern: HashMap::new(),
            #[cfg(test)]
            stats: [0; 4],
        }
    }
    pub(super) fn clear(&mut self) {
        self.coords.clear();
        self.references.clear();
        self.intern.clear();
    }
    #[cfg(test)]
    fn winner_intervals(
        raw: RawEventsView<'_>,
    ) -> Result<HashMap<(i64, NodeDigest, bool), Vec<Interval>>, String> {
        Self::winner_intervals_admitted(&super::super::audit_store::NativeAdmission(None), raw)?
            .ok_or_else(|| err("来源证书身份坐标不可固定"))
    }
    fn winner_intervals_admitted(
        s: &(impl CodecRead + ?Sized),
        raw: RawEventsView<'_>,
    ) -> Result<Option<HashMap<(i64, NodeDigest, bool), Vec<Interval>>>, String> {
        // Establish fixed identity coordinates and no cross-identity coordinate alias.
        // Future malformed input disables proof reuse without changing an earlier cut's errors.
        let mut identity_coords = BTreeMap::new();
        let mut coord_identities = BTreeMap::new();
        for event in raw.iter() {
            let identity = bj::field(event, "identity_key").as_str().unwrap_or("");
            let Ok(coord) = bj::coord(event) else {
                return Ok(None);
            };
            if !identity_coords.contains_key(identity) {
                s.native_tree_room::<&str, i64>(identity_coords.len())
                    .map_err(err)?;
            }
            if !coord_identities.contains_key(&coord) {
                s.native_tree_room::<i64, &str>(coord_identities.len())
                    .map_err(err)?;
            }
            if identity_coords
                .insert(identity, coord)
                .is_some_and(|old| old != coord)
                || coord_identities
                    .insert(coord, identity)
                    .is_some_and(|old| old != identity)
            {
                return Ok(None);
            }
        }
        struct Winner {
            revision: i64,
            ordinal: usize,
            coord: i64,
            wire: NodeDigest,
            numeric: NodeDigest,
            first: usize,
        }
        let mut winners: BTreeMap<&str, Winner> = BTreeMap::new();
        let mut out: HashMap<(i64, NodeDigest, bool), Vec<Interval>> = HashMap::new();
        let mut close = |winner: Winner, last: usize| -> Result<(), String> {
            for (numeric, d) in [(false, winner.wire), (true, winner.numeric)] {
                if !out.contains_key(&(winner.coord, d, numeric)) {
                    s.native_std_map_room(&mut out).map_err(err)?;
                }
                let runs = out.entry((winner.coord, d, numeric)).or_default();
                let interval = Interval {
                    first: winner.first,
                    last,
                };
                if let Some(prev) = runs
                    .last_mut()
                    .filter(|prev| prev.last + 1 == interval.first)
                {
                    prev.last = last;
                } else {
                    s.native_vec_room(runs, 1).map_err(err)?;
                    runs.push(interval);
                }
            }
            Ok(())
        };
        for (ordinal, event) in raw.iter().enumerate() {
            let identity = bj::field(event, "identity_key").as_str().unwrap_or("");
            let revision = bj::field(event, "revision").as_i64().unwrap_or(0);
            if winners.get(identity).is_some_and(|old| {
                !bj::effective_precedence(revision, ordinal, old.revision, old.ordinal).is_lt()
            }) {
                continue;
            }
            let Ok(wire) = bj::RawReference::new(event, false) else {
                return Ok(None);
            };
            let Ok(numeric) = bj::RawReference::new(event, true) else {
                return Ok(None);
            };
            let winner = Winner {
                revision,
                ordinal,
                coord: bj::coord(event)?,
                wire: super::super::history::hash_json_in_store(s, wire.view(), None, None)?,
                numeric: super::super::history::hash_json_in_store(s, numeric.view(), None, None)?,
                first: ordinal + 1,
            };
            if !winners.contains_key(identity) {
                s.native_tree_room::<&str, Winner>(winners.len())
                    .map_err(err)?;
            }
            if let Some(old) = winners.insert(identity, winner) {
                close(old, ordinal)?;
            }
        }
        for (_, winner) in winners {
            close(winner, raw.len())?;
        }
        Ok(Some(out))
    }
    fn share(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        values: sc::CodecVec<i64>,
    ) -> Result<Rc<[i64]>, String> {
        if let Some(shared) = self.intern.get(&*values) {
            return Ok(shared.clone());
        }
        // Rc<[i64]> = 两个引用计数 + 对齐后的 i64 数组；旧 CodecVec 仍在父账内。
        let layout = std::alloc::Layout::array::<i64>(values.len())
            .and_then(|array| {
                std::alloc::Layout::new::<[usize; 2]>()
                    .extend(array)
                    .map(|v| v.0.pad_to_align())
            })
            .map_err(|_| err(s.native_overflow()))?;
        s.native_admit(layout.size()).map_err(err)?;
        s.native_std_map_room(&mut self.intern).map_err(err)?;
        let shared: Rc<[i64]> = Rc::from(&*values);
        self.intern.insert(shared.clone(), shared.clone());
        Ok(shared)
    }
    pub(super) fn coords(
        &mut self,
        s: &mut (impl CodecRead + ?Sized),
        r: &RefValue,
        domain: ParseDomain,
    ) -> Result<SourceCoords, String> {
        let key = SequenceKey::new(r, domain, false)?;
        if !self.capture.contains(&key.digest) {
            return super::coords(s, r).map(SourceCoords::Original);
        }
        if let Some(values) = self.coords.get(&key) {
            #[cfg(test)]
            {
                self.stats[0] += 1;
                STATS.with(|s| s.set(Some(self.stats)));
            }
            return Ok(SourceCoords::Shared(values.clone()));
        }
        #[cfg(test)]
        {
            self.stats[1] += 1;
            STATS.with(|s| s.set(Some(self.stats)));
        }
        let parsed = super::coords(s, r)?;
        let values = self.share(s, parsed)?;
        s.native_std_map_room(&mut self.coords).map_err(err)?;
        self.coords.insert(key, values.clone());
        Ok(SourceCoords::Shared(values))
    }
    pub(super) fn references(
        &mut self,
        s: &mut (impl CodecRead + ?Sized),
        r: &RefValue,
        raw: &BusinessMap<i64, Source>,
        numeric: bool,
        checked: &mut BusinessMap<(NodeDigest, bool), i64>,
        prefix: usize,
        domain: ParseDomain,
    ) -> Result<SourceCoords, String> {
        if !self.intervals_ready {
            self.intervals = Self::winner_intervals_admitted(s, self.raw)?;
            self.intervals_ready = true;
        }
        let key = SequenceKey::new(r, domain, numeric)?;
        if self.intervals.is_none() || !self.capture.contains(&key.digest) {
            return super::references(s, r, raw, numeric, checked).map(SourceCoords::Original);
        }
        if let Some(proof) = self
            .references
            .get(&key)
            .filter(|proof| proof.interval.contains(prefix))
        {
            #[cfg(test)]
            {
                self.stats[2] += 1;
                STATS.with(|s| s.set(Some(self.stats)));
            }
            return Ok(SourceCoords::Shared(proof.values.clone()));
        }
        #[cfg(test)]
        {
            self.stats[3] += 1;
            STATS.with(|s| s.set(Some(self.stats)));
        }
        let values = super::references(s, r, raw, numeric, checked)?;
        let mut interval = Interval {
            first: 0,
            last: self.raw_len,
        };
        for &coord in &values {
            let source = raw
                .get(&coord)
                .ok_or_else(|| err("引用不是本cut生效来源"))?;
            let digest = if numeric { source.numeric } else { source.wire };
            let Some(run) = self
                .intervals
                .as_ref()
                .unwrap()
                .get(&(coord, digest, numeric))
                .and_then(|runs| runs.iter().find(|run| run.contains(prefix)))
            else {
                return Ok(SourceCoords::Original(values));
            };
            interval = interval.intersect(*run);
        }
        let values = self.share(s, values)?;
        s.native_std_map_room(&mut self.references).map_err(err)?;
        self.references.insert(
            key,
            Proof {
                values: values.clone(),
                interval,
            },
        );
        Ok(SourceCoords::Shared(values))
    }
}
