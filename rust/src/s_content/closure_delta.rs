//! #1647：四根严格核验的单位导出与非准入 Δ 探针。
//! Frontier 是诊断输入，不签发 U-A 或来源资格；生产入口不得用此探针替代准入。
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Context {
    Value,
    JsonInner,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum VerificationMode {
    Public,
    Template,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Unit {
    pub digest: NodeDigest,
    pub context: Context,
    pub mode: VerificationMode,
}
impl Unit {
    pub(super) fn new(digest: NodeDigest, flavor: Flavor, mode: Mode) -> Self {
        Self {
            digest,
            context: match flavor {
                Flavor::Value => Context::Value,
                Flavor::JsonInner => Context::JsonInner,
            },
            mode: match mode {
                Mode::Public => VerificationMode::Public,
                Mode::Template => VerificationMode::Template,
            },
        }
    }
}

/// 实现者只点查完整键。读集身份和 U-A 资格不由本 trait 证明。
pub trait Frontier {
    fn contains(&self, unit: Unit) -> Result<bool, CodecError>;
}
pub struct EmptyFrontier;
impl Frontier for EmptyFrontier {
    fn contains(&self, _: Unit) -> Result<bool, CodecError> {
        Ok(false)
    }
}

/// 四根全部成功后才生成；保留原预算容器，不复制历史集合或序列 memo。
pub struct Probe {
    public: WorkSet<(NodeDigest, Flavor)>,
    template: WorkSet<(NodeDigest, Flavor)>,
}
impl Probe {
    pub fn for_each_unit(&self, mut visit: impl FnMut(Unit)) {
        for (set, mode) in [
            (&self.public, Mode::Public),
            (&self.template, Mode::Template),
        ] {
            match set {
                WorkSet::Plain(seen) => {
                    for &(digest, flavor) in seen {
                        visit(Unit::new(digest, flavor, mode));
                    }
                }
                WorkSet::Owned(seen) => {
                    for &(digest, flavor) in seen {
                        visit(Unit::new(digest, flavor, mode));
                    }
                }
            }
        }
    }
}

pub const VERIFIER_REVISION: &str = "s-content-four-root-units/1";

pub struct VerifiedUnits {
    units: Probe,
    roots: [Built; 4],
}
impl VerifiedUnits {
    pub fn matches_roots(&self, roots: [RefView<'_>; 4]) -> Result<bool, CodecError> {
        for (actual, expected) in roots.into_iter().zip(&self.roots) {
            if actual.format() != CONTENT_FORMAT
                || actual.digest()? != expected.digest
                || actual.kind() != expected.kind
                || actual.extent() != expected.extent
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub fn try_for_each_unit<E>(
        &self,
        mut visit: impl FnMut(Unit) -> Result<(), E>,
    ) -> Result<(), E> {
        for (set, mode) in [
            (&self.units.public, Mode::Public),
            (&self.units.template, Mode::Template),
        ] {
            match set {
                WorkSet::Plain(seen) => {
                    for &(digest, flavor) in seen {
                        visit(Unit::new(digest, flavor, mode))?;
                    }
                }
                WorkSet::Owned(seen) => {
                    for &(digest, flavor) in seen {
                        visit(Unit::new(digest, flavor, mode))?;
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn verify_four_roots(
    source: &mut dyn NodeSource,
    roots: [RefView<'_>; 4],
) -> Result<VerifiedUnits, CodecError> {
    let units = verify_roots(source, roots, None)?;
    let root = |r: RefView<'_>| -> Result<Built, CodecError> {
        Ok(Built {
            digest: r.digest()?,
            kind: r.kind(),
            extent: r.extent(),
        })
    };
    Ok(VerifiedUnits {
        units,
        roots: [
            root(roots[0])?,
            root(roots[1])?,
            root(roots[2])?,
            root(roots[3])?,
        ],
    })
}

/// 顺序固定 scope、generated_state、publication_template、delta_template。
/// 根四字段每次重核。任何失败直接返回；不会回退完整核验或返回部分单位。
pub fn probe_four_roots(
    source: &mut dyn NodeSource,
    roots: [RefView<'_>; 4],
    frontier: &dyn Frontier,
) -> Result<Probe, CodecError> {
    verify_roots(source, roots, Some(frontier))
}
fn verify_roots(
    source: &mut dyn NodeSource,
    roots: [RefView<'_>; 4],
    frontier: Option<&dyn Frontier>,
) -> Result<Probe, CodecError> {
    let mut public = Verifier::new(source, Mode::Public);
    public.closure_frontier = frontier;
    public.verify_refview(roots[0])?;
    public.verify_refview(roots[1])?;
    let public = public.into_memo().seen;
    let mut template = Verifier::new(source, Mode::Template);
    template.closure_frontier = frontier;
    template.verify_refview(roots[2])?;
    template.verify_refview(roots[3])?;
    let template = template.into_memo().seen;
    Ok(Probe { public, template })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    impl Frontier for BTreeSet<Unit> {
        fn contains(&self, unit: Unit) -> Result<bool, CodecError> {
            Ok(self.contains(&unit))
        }
    }
    fn units(p: Probe) -> BTreeSet<Unit> {
        let mut result = BTreeSet::new();
        p.for_each_unit(|u| {
            result.insert(u);
        });
        result
    }
    fn probe(
        nodes: &mut MemoryNodes,
        refs: &[ContentRef; 4],
        old: &dyn Frontier,
    ) -> Result<Probe, CodecError> {
        probe_four_roots(nodes, refs.each_ref().map(Into::into), old)
    }
    fn full(nodes: &mut MemoryNodes, refs: &[ContentRef; 4]) -> Result<(), CodecError> {
        for (i, r) in refs.iter().enumerate() {
            verify_refview(
                nodes,
                r.into(),
                if i < 2 { Mode::Public } else { Mode::Template },
            )?;
        }
        Ok(())
    }
    #[test]
    fn p6_verified_export_matches_strict_units_and_binds_all_root_fields() {
        let mut nodes = MemoryNodes::new();
        let root = write_value(&mut nodes, &Value::String("exported".into()))
            .unwrap()
            .content_ref();
        let roots: [ContentRef; 4] = std::array::from_fn(|_| root.clone());
        let expected = units(probe(&mut nodes, &roots, &EmptyFrontier).unwrap());
        let verified = verify_four_roots(&mut nodes, roots.each_ref().map(Into::into)).unwrap();
        let mut exported = BTreeSet::new();
        verified
            .try_for_each_unit(|unit| {
                exported.insert(unit);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(exported, expected);
        assert!(verified
            .matches_roots(roots.each_ref().map(Into::into))
            .unwrap());
        for index in 0..4 {
            let mut changed = roots.clone();
            changed[index].extent = Some(u64::MAX);
            assert!(!verified
                .matches_roots(changed.each_ref().map(Into::into))
                .unwrap());
            assert!(verify_four_roots(&mut nodes, changed.each_ref().map(Into::into)).is_err());
        }
        let mut calls = 0;
        assert!(verified
            .try_for_each_unit(|_| {
                calls += 1;
                Err::<(), _>(())
            })
            .is_err());
        assert_eq!(calls, 1);
    }

    #[test]
    fn p6_closure_units_partition_modes_and_exclude_sequences() {
        let mut n = MemoryNodes::new();
        let r = write_value(&mut n, &Value::String("hello".into()))
            .unwrap()
            .content_ref();
        let refs = std::array::from_fn(|_| r.clone());
        let got = units(probe(&mut n, &refs, &EmptyFrontier).unwrap());
        assert_eq!(got.len(), 2);
        assert!(got.iter().all(|u| u.digest == r.digest().unwrap()));
        assert!(units(probe(&mut n, &refs, &got).unwrap()).is_empty());
    }
    #[test]
    fn p6_closure_root_declarations_are_checked_even_at_frontier() {
        let mut n = MemoryNodes::new();
        let r = write_value(&mut n, &Value::String("root".into()))
            .unwrap()
            .content_ref();
        let refs = std::array::from_fn(|_| r.clone());
        let prior = units(probe(&mut n, &refs, &EmptyFrontier).unwrap());
        for position in 0..4 {
            for field in 0..4 {
                let mut bad = refs.clone();
                match field {
                    0 => bad[position].format = "wrong".into(),
                    1 => bad[position].root = "not-a-root".into(),
                    2 => bad[position].kind = Kind::Null,
                    _ => bad[position].extent = Some(999),
                }
                assert_eq!(
                    probe(&mut n, &bad, &prior).err().unwrap().code(),
                    full(&mut n, &bad).unwrap_err().code()
                );
            }
        }
    }
    #[test]
    fn p6_closure_old_unreachable_node_is_new_when_reachable() {
        let mut n = MemoryNodes::new();
        let old = write_value(&mut n, &Value::Null).unwrap().content_ref();
        let latent = write_value(&mut n, &Value::Bool(true))
            .unwrap()
            .content_ref();
        let refs = std::array::from_fn(|_| old.clone());
        let prior = units(probe(&mut n, &refs, &EmptyFrontier).unwrap());
        let next = write_value(
            &mut n,
            &Value::Array(vec![
                Value::edge_of_ref(&old).unwrap(),
                Value::edge_of_ref(&latent).unwrap(),
            ]),
        )
        .unwrap()
        .content_ref();
        let refs = std::array::from_fn(|_| next.clone());
        let added = units(probe(&mut n, &refs, &prior).unwrap());
        assert_eq!(added.len(), 4);
        assert_eq!(
            added
                .iter()
                .filter(|u| u.digest == latent.digest().unwrap())
                .count(),
            2
        );
        assert!(added.is_disjoint(&prior));
    }
    #[test]
    fn p6_closure_context_and_mode_never_borrow_other_proofs() {
        let mut n = MemoryNodes::new();
        let text = write_value(&mut n, &Value::JsonText(Box::new(JsonValue::Null)))
            .unwrap()
            .content_ref();
        let refs = std::array::from_fn(|_| text.clone());
        let prior = units(probe(&mut n, &refs, &EmptyFrontier).unwrap());
        let nested = frame(tag::JSON_TEXT, &text.digest().unwrap());
        n.insert_unchecked(nested, tag::JSON_TEXT, text.digest().unwrap().to_vec());
        let bad = ContentRef {
            format: CONTENT_FORMAT.into(),
            root: root_string(&nested),
            kind: Kind::JsonText,
            extent: None,
        };
        let refs = std::array::from_fn(|_| bad.clone());
        assert_eq!(
            probe(&mut n, &refs, &prior).err().unwrap().code(),
            full(&mut n, &refs).unwrap_err().code()
        );
        let batch = write_template(&mut n, &Value::BatchSelf)
            .unwrap()
            .content_ref();
        let mut prior = BTreeSet::new();
        prior.insert(Unit::new(
            batch.digest().unwrap(),
            Flavor::Value,
            Mode::Template,
        ));
        let refs = std::array::from_fn(|_| batch.clone());
        assert_eq!(
            probe(&mut n, &refs, &prior).err().unwrap().code(),
            code::TEMPLATE_PLACEHOLDER
        );
    }
    #[test]
    fn p6_closure_missing_corrupt_and_lookup_errors_return_no_probe() {
        let mut n = MemoryNodes::new();
        let r = write_value(&mut n, &Value::Array(vec![Value::Edge([19; 32])]))
            .unwrap()
            .content_ref();
        let refs = std::array::from_fn(|_| r.clone());
        assert_eq!(
            probe(&mut n, &refs, &EmptyFrontier).err().unwrap().code(),
            full(&mut n, &refs).unwrap_err().code()
        );
        n.insert_unchecked(r.digest().unwrap(), tag::NULL, vec![]);
        assert_eq!(
            probe(&mut n, &refs, &EmptyFrontier).err().unwrap().code(),
            full(&mut n, &refs).unwrap_err().code()
        );
        struct Broken;
        impl Frontier for Broken {
            fn contains(&self, _: Unit) -> Result<bool, CodecError> {
                Err(CodecError::new(code::SOURCE, "frontier unavailable"))
            }
        }
        let r = write_value(&mut n, &Value::Null).unwrap().content_ref();
        let refs = std::array::from_fn(|_| r.clone());
        assert_eq!(
            probe(&mut n, &refs, &Broken).err().unwrap().code(),
            code::SOURCE
        );
        full(&mut n, &refs).unwrap();
    }
    #[test]
    fn p6_closure_shared_byte_tail_keeps_utf8_entry_state() {
        let mut n = MemoryNodes::new();
        let value = format!("{}é", "a".repeat(CHUNK_BYTES - 1));
        let valid = write_value(&mut n, &Value::String(value))
            .unwrap()
            .content_ref();
        let tail = frame(tag::BYTES_CHUNK, &[0xa9]);
        assert!(n.contains(&tail));
        let leaf = frame(tag::SEQ_LEAF, &tail);
        assert!(n.contains(&leaf));
        verify(&mut n, &valid).unwrap();
        let mut body = 1u64.to_be_bytes().to_vec();
        body.extend_from_slice(&leaf);
        let digest = frame(tag::STRING, &body);
        n.insert_unchecked(digest, tag::STRING, body);
        let invalid = ContentRef::new(Kind::String, &digest, Some(1));
        let refs = [valid.clone(), invalid, valid.clone(), valid];
        assert_eq!(
            probe(&mut n, &refs, &EmptyFrontier).err().unwrap().code(),
            code::UTF8
        );
        assert_eq!(full(&mut n, &refs).unwrap_err().code(), code::UTF8);
    }
    #[test]
    fn p6_closure_probe_retains_and_releases_parent_accounting() {
        use capture_allocation::scratch::{
            CodecWork, ScratchCharge, ScratchFailure, ScratchParent, WorkScope,
        };
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        struct Parent(AtomicUsize);
        impl ScratchParent for Parent {
            fn health(&self) -> Result<(), ScratchFailure> {
                Ok(())
            }
            fn invalidate(&self) {
                panic!("unexpected invalidation");
            }
            fn reserve(&self, n: usize, _: ScratchCharge) -> Result<u64, ScratchFailure> {
                self.0.fetch_add(n, Ordering::SeqCst);
                Ok(0)
            }
            fn release(&self, n: usize, _: ScratchCharge, _: u64) -> Result<(), ScratchFailure> {
                assert!(self.0.fetch_sub(n, Ordering::SeqCst) >= n);
                Ok(())
            }
        }
        struct Scoped<'a> {
            nodes: &'a mut MemoryNodes,
            scope: &'a WorkScope,
        }
        impl NodeSource for Scoped<'_> {
            fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> {
                self.nodes.read(d)
            }
            fn codec_work(&self) -> Option<&WorkScope> {
                Some(self.scope)
            }
        }
        let parent = Arc::new(Parent(AtomicUsize::new(0)));
        let mut n = MemoryNodes::new();
        let r = write_value(&mut n, &Value::String("charged".into()))
            .unwrap()
            .content_ref();
        CodecWork::new(parent.clone(), 1024 * 1024)
            .unwrap()
            .run(|scope| {
                let mut source = Scoped {
                    nodes: &mut n,
                    scope,
                };
                let refs = std::array::from_fn(|_| (&r).into());
                let p = probe_four_roots(&mut source, refs, &EmptyFrontier)?;
                assert!(scope.snapshot().live > 0);
                drop(p);
                assert_eq!(scope.snapshot().live, 0);
                Ok::<_, CodecError>(())
            })
            .unwrap();
        assert_eq!(parent.0.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn p6_closure_reused_object_key_is_not_a_new_unit_and_still_requires_string() {
        let mut n = MemoryNodes::new();
        let first = write_value(&mut n, &Value::Object(vec![("k".into(), Value::Null)]))
            .unwrap()
            .content_ref();
        let refs = std::array::from_fn(|_| first.clone());
        let prior = units(probe(&mut n, &refs, &EmptyFrontier).unwrap());
        assert_eq!(prior.len(), 6);
        let next = write_value(
            &mut n,
            &Value::Object(vec![("k".into(), Value::Bool(true))]),
        )
        .unwrap()
        .content_ref();
        let refs = std::array::from_fn(|_| next.clone());
        let added = units(probe(&mut n, &refs, &prior).unwrap());
        assert_eq!(added.len(), 4);
        assert!(added.is_disjoint(&prior));
        full(&mut n, &refs).unwrap();
        let null = frame(tag::NULL, &[]);
        let mut field_body = null.to_vec();
        field_body.extend_from_slice(&null);
        let field = frame(tag::OBJECT_FIELD, &field_body);
        n.insert_unchecked(field, tag::OBJECT_FIELD, field_body);
        let leaf = frame(tag::SEQ_LEAF, &field);
        n.insert_unchecked(leaf, tag::SEQ_LEAF, field.to_vec());
        let mut body = 1u64.to_be_bytes().to_vec();
        body.extend_from_slice(&leaf);
        let digest = frame(tag::OBJECT, &body);
        n.insert_unchecked(digest, tag::OBJECT, body);
        let bad = ContentRef::new(Kind::Object, &digest, Some(1));
        let refs = std::array::from_fn(|_| bad.clone());
        assert_eq!(
            probe(&mut n, &refs, &prior).err().unwrap().code(),
            code::VALUE_KIND
        );
        assert_eq!(full(&mut n, &refs).unwrap_err().code(), code::VALUE_KIND);
    }
}
