//! #1372：仅缓存精确原字节解码与已验证结构前缀。每次仍在当前事务读全域。
//! 不使用 generation/mtime/data_version 代替依赖校验；默认禁用，serve 显式配置容量。
use super::*;
use std::{cell::RefCell, sync::Arc};

pub(super) type DeltaRow = (
    i64,
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    String,
    String,
    String,
);
type Identity = (String, String, (String, String));
struct Root {
    rows: Vec<DeltaRow>,
    identity: Identity,
    walk: audit::AuditWalk,
    bytes: usize,
}
struct Memo {
    limit: usize,
    batches: BTreeMap<String, (Arc<[u8]>, Arc<Value>)>,
    batch_bytes: usize,
    root: Option<Root>,
}
impl Default for Memo {
    fn default() -> Self {
        #[cfg(test)]
        let limit = std::env::var("S_TEST_AUDIT_CACHE_SOURCE_BYTES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        #[cfg(not(test))]
        let limit = 0;
        Self {
            limit,
            batches: BTreeMap::new(),
            batch_bytes: 0,
            root: None,
        }
    }
}
#[cfg(test)]
thread_local! {static LAST_RESUMED: std::cell::Cell<usize> = const {std::cell::Cell::new(0)};}
#[cfg(test)]
pub(super) fn last_resumed() -> usize {
    LAST_RESUMED.with(|v| v.get())
}
thread_local! { static MEMO: RefCell<Memo> = RefCell::new(Memo::default()); }
pub(super) fn configure(limit: usize) {
    MEMO.with(|m| {
        *m.borrow_mut() = Memo {
            limit,
            ..Memo::default()
        }
    });
}
pub(super) fn decoded(id: &str, bytes: &[u8]) -> Option<(Arc<[u8]>, Arc<Value>)> {
    MEMO.with(|m| {
        m.borrow()
            .batches
            .get(id)
            .filter(|(old, _)| old.as_ref() == bytes)
            .cloned()
    })
}
pub(super) fn store_batches(batches: BTreeMap<String, (Arc<[u8]>, Arc<Value>)>) {
    // 原字节与其解码值各记一份规范源字节；此上限不是 allocator/RSS 上限。
    let size = batches.iter().fold(0usize, |n, (id, (b, _))| {
        n.saturating_add(id.len())
            .saturating_add(b.len().saturating_mul(2))
    });
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if size.saturating_add(m.root.as_ref().map_or(0, |r| r.bytes)) > m.limit || m.limit == 0 {
            let limit = m.limit;
            *m = Memo {
                limit,
                ..Memo::default()
            };
        } else {
            m.batches = batches;
            m.batch_bytes = size;
        }
    });
}
pub(super) fn resume(rows: &[DeltaRow], identity: &Identity, walk: &mut audit::AuditWalk) -> usize {
    #[cfg(test)]
    LAST_RESUMED.with(|v| v.set(0));
    MEMO.with(|m| {
        let m = m.borrow();
        let Some(old) = &m.root else { return 0 };
        if &old.identity != identity
            || rows.len() < old.rows.len()
            || rows[..old.rows.len()] != old.rows
        {
            return 0;
        }
        let generation = old.rows.len() as i64;
        if walk.resume_from(&old.walk, generation) {
            #[cfg(test)]
            LAST_RESUMED.with(|v| v.set(old.rows.len()));
            old.rows.len()
        } else {
            0
        }
    })
}
pub(super) fn store_root(rows: Vec<DeltaRow>, identity: Identity, walk: audit::AuditWalk) {
    let row_bytes = rows.iter().fold(0usize, |n, r| {
        n.saturating_add(
            r.1.len()
                + r.2.len()
                + r.3.len()
                + r.4.len()
                + r.5.len()
                + r.6.len()
                + r.8.len()
                + r.9.len()
                + r.10.len()
                + 16,
        )
    });
    let bytes = row_bytes
        .saturating_add(walk.source_bytes())
        .saturating_add(
            identity.0.len() + identity.1.len() + identity.2 .0.len() + identity.2 .1.len(),
        );
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if bytes.saturating_add(m.batch_bytes) > m.limit || m.limit == 0 {
            let limit = m.limit;
            *m = Memo {
                limit,
                ..Memo::default()
            };
        } else {
            m.root = Some(Root {
                rows,
                identity,
                walk,
                bytes,
            });
        }
    });
}
