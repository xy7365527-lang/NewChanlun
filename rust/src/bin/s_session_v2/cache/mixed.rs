//! #1527：已核旧审计前缀，同一缓存上限与 mixed 准入事务。
use super::*;
use crate::v3::audit_budget::LegacyObserver;
use crate::v3::business_json::MixedJsonError;
use newchan_rust::s_content::{capture_allocation::native_owned::NativeValue, CodecVec};
pub(crate) type Text = NativeValue<String>;
pub(crate) type Identity = [Text; 6];
pub(crate) type Originals = NativeTree<Text, Option<Arc<RootCert>>>;

#[derive(PartialEq, Eq)]
pub(crate) struct DeltaRow {
    pub(crate) generation: i64,
    pub(crate) input_frontier: i64,
    /// session, catalog, base, next, seq_range, index_frontier, run_status, evidence, delta, digest.
    pub(crate) text: [Text; 10],
}
pub(super) struct Root {
    pub(super) rows: CodecVec<DeltaRow>,
    identity: Identity,
    originals: NativeTree<Text, Option<Weak<()>>>,
    walk: audit::mixed::AuditWalk,
    pub(super) bytes: usize,
}
fn text(value: &str, work: &WorkScope) -> Result<Text, MixedJsonError> {
    Text::try_from_str(value, Some(work)).map_err(MixedJsonError::Resource)
}
pub(crate) fn resume(
    rows: &[DeltaRow],
    identity: &Identity,
    walk: &mut audit::mixed::AuditWalk,
    originals: &Originals,
    work: &WorkScope,
    observer: &dyn LegacyObserver,
) -> Result<usize, MixedJsonError> {
    observer
        .check_work(work)
        .map_err(MixedJsonError::Resource)?;
    #[cfg(test)]
    LAST_RESUMED.with(|v| v.set(0));
    MEMO.with(|memo| {
        let memo = memo.borrow();
        let Some(super::Root::Mixed(old)) = &memo.root else {
            return Ok(0);
        };
        old.check_retained_source()
            .map_err(MixedJsonError::Resource)?;
        if &old.identity != identity
            || rows.len() < old.rows.len()
            || rows[..old.rows.len()] != old.rows[..]
        {
            return Ok(0);
        }
        let graph3 = identity[5].as_str() == evidence_store::GRAPH_FORMAT;
        let graph = graph3 || identity[5].as_str() == evidence_store::GRAPH_FORMAT_V2;
        if graph {
            for row in &old.rows {
                observer.precheck().map_err(MixedJsonError::Resource)?;
                let batch =
                    Text::try_format(format_args!("batch/{}", row.text[5].as_ref()), Some(work))
                        .map_err(MixedJsonError::Resource)?;
                if !old.originals.contains_key(batch.as_str()) {
                    return Ok(0);
                }
                if graph3 {
                    let delta =
                        Text::try_format(format_args!("delta/{}", row.generation), Some(work))
                            .map_err(MixedJsonError::Resource)?;
                    if !old.originals.contains_key(delta.as_str()) {
                        return Ok(0);
                    }
                }
            }
        }
        for (key, old_cert) in old.originals.iter() {
            observer.precheck().map_err(MixedJsonError::Resource)?;
            match (old_cert, originals.get(key.as_str())) {
                (Some(a), Some(Some(b)))
                    if Weak::ptr_eq(a, &b.identity_weak())
                        && Weak::ptr_eq(&b.lineage, &Arc::downgrade(&memo.cert_lineage)) => {}
                _ => return Ok(0),
            }
        }
        if walk.resume_from(&old.walk, old.rows.len() as i64, work, observer)? {
            #[cfg(test)]
            LAST_RESUMED.with(|v| v.set(old.rows.len()));
            Ok(old.rows.len())
        } else {
            Ok(0)
        }
    })
}
pub(crate) fn store_root(
    rows: CodecVec<DeltaRow>,
    identity: Identity,
    walk: audit::mixed::AuditWalk,
    originals: &Originals,
    work: &WorkScope,
    observer: &dyn LegacyObserver,
) -> Result<(), MixedJsonError> {
    observer
        .check_work(work)
        .map_err(MixedJsonError::Resource)?;
    MEMO.with(|memo| {
        let mut memo = memo.borrow_mut();
        if memo.limit == 0 {
            *memo = Memo::disabled();
            return Ok(());
        }
        let mut bytes = rows.iter().fold(0usize, |total, row| {
            row.text
                .iter()
                .fold(total.saturating_add(16), |total, text| {
                    total.saturating_add(text.len())
                })
        });
        for key in originals.keys() {
            bytes = bytes.saturating_add(key.len()).saturating_add(16);
        }
        bytes = bytes.saturating_add(walk.source_bytes(work, observer)?);
        for value in &identity {
            bytes = bytes.saturating_add(value.len());
        }
        if bytes > memo.limit {
            let keep = match &memo.root {
                Some(super::Root::Mixed(old)) => {
                    old.bytes <= memo.limit
                        && old.identity == identity
                        && rows.len() >= old.rows.len()
                        && rows[..old.rows.len()] == old.rows[..]
                }
                _ => false,
            };
            if !keep {
                memo.root = None;
            }
            memo.fit_retention_typed(Some(work))
                .map_err(MixedJsonError::Resource)?;
            return Ok(());
        }
        let mut proofs = NativeTree::default();
        for (key, cert) in originals.iter() {
            proofs
                .insert_admitted(
                    text(key, work)?,
                    cert.as_ref().map(|cert| cert.identity_weak()),
                    &NativeAdmission(Some(work)),
                )
                .map_err(MixedJsonError::Resource)?;
        }
        let root = Arc::try_new_shallow(
            Root {
                rows,
                identity,
                originals: proofs,
                walk,
                bytes,
            },
            Some(work),
        )
        .map_err(MixedJsonError::Resource)?;
        memo.root = Some(super::Root::Mixed(root));
        memo.fit_retention_typed(Some(work))
            .map_err(MixedJsonError::Resource)?;
        Ok(())
    })
}
