//! #1372：仅缓存精确原字节解码与已验证结构前缀。每次仍在当前事务读全域。
//! 不使用 generation/mtime/data_version 代替依赖校验；默认禁用，serve 显式配置容量。
//! #1394：root（已验证前缀）与 decoded 共用同一个原始总量上限，root 优先——装入新 root 前按
//! 剩余预算驱逐 decoded；新 root 单独超限时保留仍可复用的旧前缀，不清空整个 Memo；decoded 只保留
//! 本次 capture 提供的集合中装得下的条目，单个过大者跳过。该上限是规范源字节记账（原字节 + 等额
//! 解码规范源字节 + id + root rows/walk/identity），不是 allocator/RSS 上限，也不声称 RSS 受其限制。
use super::*;
use std::collections::BTreeSet;
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
/// （session_id、catalog_revision、（profile_id、profile_hash）、存储格式标签）。
/// 末项是 #1394 的私有 Delta 域：模式变化必须使旧 root/证书失效，不跨模式复用。
type Identity = (String, String, (String, String), String);
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
impl Memo {
    /// limit=0：完全禁用，root 与 decoded 都不留。
    fn disabled() -> Self {
        Self {
            limit: 0,
            batches: BTreeMap::new(),
            batch_bytes: 0,
            root: None,
        }
    }
    #[cfg(test)]
    fn total_bytes(&self) -> usize {
        self.root
            .as_ref()
            .map_or(0, |r| r.bytes)
            .saturating_add(self.batch_bytes)
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
/// decoded 条目记账：原字节 + 等额解码规范源字节 + id（与 root 同一总量口径）。
fn batch_cost(id_len: usize, bytes_len: usize) -> usize {
    id_len.saturating_add(bytes_len.saturating_mul(2))
}
fn batches_cost(batches: &BTreeMap<String, (Arc<[u8]>, Arc<Value>)>) -> usize {
    batches.iter().fold(0usize, |n, (id, (b, _))| {
        n.saturating_add(batch_cost(id.len(), b.len()))
    })
}
/// 在预算内挑选可保留的 decoded 条目：优先仍在本次输入集合中的现有缓存，其余按 id 序尽量装入；
/// 单个装不下的条目跳过，不因此清空其他条目。
fn select_batches(
    incoming: &BTreeMap<String, (Arc<[u8]>, Arc<Value>)>,
    warm: &BTreeSet<String>,
    budget: usize,
) -> (BTreeMap<String, (Arc<[u8]>, Arc<Value>)>, usize) {
    let mut kept = BTreeMap::new();
    let mut used = 0usize;
    for warm_pass in [true, false] {
        for (id, entry) in incoming {
            if warm.contains(id) != warm_pass {
                continue;
            }
            let cost = batch_cost(id.len(), entry.0.len());
            if cost > budget.saturating_sub(used) {
                continue;
            }
            used = used.saturating_add(cost);
            kept.insert(id.clone(), entry.clone());
        }
    }
    (kept, used)
}
/// root 优先的驱逐：先丢最大的条目，尽量保留更多条目；同大小按 id 降序，保证确定性。
fn trim_batches(m: &mut Memo, budget: usize) {
    if m.batch_bytes <= budget {
        return;
    }
    let mut order = m
        .batches
        .iter()
        .map(|(id, (b, _))| (batch_cost(id.len(), b.len()), id.clone()))
        .collect::<Vec<_>>();
    order.sort_by(|a, b| b.cmp(a));
    let mut over = m.batch_bytes - budget;
    for (cost, id) in order {
        if over == 0 {
            break;
        }
        m.batches.remove(&id);
        m.batch_bytes = m.batch_bytes.saturating_sub(cost);
        over = over.saturating_sub(cost);
    }
    m.batch_bytes = batches_cost(&m.batches);
}
pub(super) fn store_batches(batches: BTreeMap<String, (Arc<[u8]>, Arc<Value>)>) {
    // 原字节与其解码值各记一份规范源字节；此上限不是 allocator/RSS 上限。
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if m.limit == 0 {
            *m = Memo::disabled();
            return;
        }
        // root 优先：上限被调小时先放弃不再装得下的旧 root，使总量仍受单一上限约束。
        if m.root.as_ref().is_some_and(|r| r.bytes > m.limit) {
            m.root = None;
        }
        let root_bytes = m.root.as_ref().map_or(0, |r| r.bytes);
        let budget = m.limit.saturating_sub(root_bytes);
        let warm = m.batches.keys().cloned().collect::<BTreeSet<_>>();
        // 只接受本次 capture 提供的集合；移出可达集的旧缓存不会复活。
        let (kept, used) = select_batches(&batches, &warm, budget);
        m.batches = kept;
        m.batch_bytes = used;
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
    let bytes = root_cost(&rows, &identity, &walk);
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if m.limit == 0 {
            *m = Memo::disabled();
            return;
        }
        if bytes > m.limit {
            // 新 root 单独超限：保留此前仍可复用的旧前缀（仍在当前上限内、身份相同、且仍是当前行的
            // 前缀），不清空 decoded 缓存；判不出仍有用时只丢旧 root。
            let keep = m.root.as_ref().is_some_and(|old| {
                old.bytes <= m.limit
                    && old.identity == identity
                    && rows.len() >= old.rows.len()
                    && rows[..old.rows.len()] == old.rows
            });
            if !keep {
                m.root = None;
            }
            return;
        }
        // root 优先装入，再按剩余预算驱逐 decoded 条目（不因新 root 变大而整块清空）。
        m.root = Some(Root {
            rows,
            identity,
            walk,
            bytes,
        });
        let budget = m.limit.saturating_sub(bytes);
        trim_batches(&mut m, budget);
    });
}

/// root 记账：行内字符串 + 每行 16B + walk 的规范源字节 + identity 字段；全程饱和加，避免溢出。
fn root_cost(rows: &[DeltaRow], identity: &Identity, walk: &audit::AuditWalk) -> usize {
    let row_bytes = rows.iter().fold(0usize, |n, r| {
        n.saturating_add(
            r.1.len()
                .saturating_add(r.2.len())
                .saturating_add(r.3.len())
                .saturating_add(r.4.len())
                .saturating_add(r.5.len())
                .saturating_add(r.6.len())
                .saturating_add(r.8.len())
                .saturating_add(r.9.len())
                .saturating_add(r.10.len())
                .saturating_add(16),
        )
    });
    row_bytes
        .saturating_add(walk.source_bytes())
        .saturating_add(
            identity
                .0
                .len()
                .saturating_add(identity.1.len())
                .saturating_add(identity.2 .0.len())
                .saturating_add(identity.2 .1.len())
                .saturating_add(identity.3.len()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空索引 walk：缓存不改动审计捕获，故测试只需最小合法 walk 即可组装 Root。
    fn empty_walk() -> audit::AuditWalk {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(crate::SCHEMA).unwrap();
        conn.execute_batch(
            "INSERT INTO meta(key,value) VALUES \
             ('generation','0'),('structure_cut','cut-0'),\
             ('catalog_revision','s2-axis-quantifiers+new-bi/1'),\
             ('scope','{\"structure\":\"CompleteCut\",\"economic\":\"not_started\"}')",
        )
        .unwrap();
        audit::AuditWalk::capture(&conn).unwrap()
    }
    fn identity(session: &str) -> Identity {
        (
            session.to_string(),
            "catalog-hash".to_string(),
            ("profile-id".to_string(), "profile-hash".to_string()),
            delta_storage::CHANGES_ONLY.to_string(),
        )
    }
    fn rows(n: i64) -> Vec<DeltaRow> {
        (1..=n)
            .map(|g| {
                (
                    g,
                    "test-c".to_string(),
                    "catalog-hash".to_string(),
                    format!("cut-{}", g - 1),
                    format!("cut-{g}"),
                    "{\"from\":\"0\",\"to\":\"0\"}".to_string(),
                    format!("batch-{g}"),
                    g - 1,
                    "not_run".to_string(),
                    "{}".to_string(),
                    format!("{{\"generation\":\"{g}\"}}"),
                )
            })
            .collect()
    }
    fn entry(id: &str, len: usize) -> (Arc<[u8]>, Arc<Value>) {
        (
            Arc::<[u8]>::from(vec![b'x'; len]),
            Arc::new(serde_json::json!({"batch_id": id})),
        )
    }
    fn batch_map(ids: &[(&str, usize)]) -> BTreeMap<String, (Arc<[u8]>, Arc<Value>)> {
        ids.iter()
            .map(|(id, len)| ((*id).to_string(), entry(id, *len)))
            .collect()
    }
    fn memo_root_rows() -> Option<usize> {
        MEMO.with(|m| m.borrow().root.as_ref().map(|r| r.rows.len()))
    }
    fn memo_batch_ids() -> Vec<String> {
        MEMO.with(|m| m.borrow().batches.keys().cloned().collect())
    }
    fn memo_total() -> usize {
        MEMO.with(|m| m.borrow().total_bytes())
    }

    #[test]
    fn root_survives_and_decoded_fills_only_the_shared_budget() {
        let base = rows(2);
        let root = root_cost(&base, &identity("test-c"), &empty_walk());
        let one = batch_cost("batch-a".len(), 64);
        let limit = root + one * 3;
        configure(limit);
        store_root(base, identity("test-c"), empty_walk());
        store_batches(batch_map(&[
            ("batch-a", 64),
            ("batch-bb", 1 << 20),
            ("batch-c", 64),
            ("batch-d", 64),
        ]));
        // root 优先保留（旧行为是整块清空），单个过大条目跳过而不牵连其他条目。
        assert_eq!(memo_root_rows(), Some(2));
        assert_eq!(memo_batch_ids(), vec!["batch-a", "batch-c", "batch-d"]);
        assert_eq!(memo_total(), root + one * 3);
        assert!(memo_total() <= limit);
        assert!(decoded("batch-a", &vec![b'x'; 64]).is_some());
        assert!(decoded("batch-bb", &vec![b'x'; 1 << 20]).is_none());
    }

    #[test]
    fn growing_root_evicts_decoded_instead_of_dropping_everything() {
        let small = rows(2);
        let big = rows(6);
        let small_bytes = root_cost(&small, &identity("test-c"), &empty_walk());
        let big_bytes = root_cost(&big, &identity("test-c"), &empty_walk());
        let one = batch_cost("batch-a".len(), 64);
        let limit = big_bytes + one;
        assert!(big_bytes >= small_bytes + one * 2);
        configure(limit);
        store_root(small, identity("test-c"), empty_walk());
        store_batches(batch_map(&[("batch-a", 64), ("batch-b", 64)]));
        assert_eq!(memo_batch_ids(), vec!["batch-a", "batch-b"]);
        store_root(big, identity("test-c"), empty_walk());
        assert_eq!(memo_root_rows(), Some(6));
        assert_eq!(memo_batch_ids(), vec!["batch-a"]); // decoded 部分驱逐，root 保留
        assert!(memo_total() <= limit);
    }

    #[test]
    fn oversized_new_root_keeps_previous_verified_prefix_usable() {
        let small = rows(2);
        let huge = rows(64);
        let small_bytes = root_cost(&small, &identity("test-c"), &empty_walk());
        let one = batch_cost("batch-a".len(), 64);
        let limit = small_bytes + one;
        assert!(root_cost(&huge, &identity("test-c"), &empty_walk()) > limit);
        configure(limit);
        store_root(small, identity("test-c"), empty_walk());
        store_batches(batch_map(&[("batch-a", 64)]));
        store_root(huge.clone(), identity("test-c"), empty_walk());
        // 装不下新 root：旧前缀与 decoded 缓存都保留，不清整个 Memo。
        assert_eq!(memo_root_rows(), Some(2));
        assert_eq!(memo_batch_ids(), vec!["batch-a"]);
        assert!(memo_total() <= limit);
        assert_eq!(resume(&huge, &identity("test-c"), &mut empty_walk()), 2);
        assert_eq!(last_resumed(), 2);
        // 前缀不再匹配时必须作废，不留死前缀、不误用。
        let mut changed = huge.clone();
        changed[0].6 = "batch-9".to_string();
        assert_eq!(resume(&changed, &identity("test-c"), &mut empty_walk()), 0);
        assert_eq!(last_resumed(), 0);
    }

    #[test]
    fn decoded_requires_identical_bytes_and_identity_and_prefix_are_checked() {
        configure(4096);
        store_batches(batch_map(&[("batch-a", 64)]));
        assert!(decoded("batch-a", &vec![b'x'; 64]).is_some());
        assert!(decoded("batch-a", &vec![b'y'; 64]).is_none()); // 同 id 不同 bytes 不命中
        store_root(rows(3), identity("test-c"), empty_walk());
        assert_eq!(resume(&rows(5), &identity("test-c"), &mut empty_walk()), 3);
        assert_eq!(resume(&rows(5), &identity("other-c"), &mut empty_walk()), 0);
        let mut changed = rows(5);
        changed[1].4 = "cut-99".to_string();
        assert_eq!(resume(&changed, &identity("test-c"), &mut empty_walk()), 0);
        assert_eq!(resume(&rows(2), &identity("test-c"), &mut empty_walk()), 0);
    }

    #[test]
    fn zero_limit_disables_cache_entirely() {
        configure(0);
        store_batches(batch_map(&[("batch-a", 64)]));
        store_root(rows(2), identity("test-c"), empty_walk());
        assert_eq!(memo_root_rows(), None);
        assert!(memo_batch_ids().is_empty());
        assert_eq!(memo_total(), 0);
        assert!(decoded("batch-a", &vec![b'x'; 64]).is_none());
        assert_eq!(resume(&rows(2), &identity("test-c"), &mut empty_walk()), 0);
        assert_eq!(last_resumed(), 0);
    }

    #[test]
    fn decoded_cache_that_left_the_reachable_set_is_not_revived() {
        configure(4096);
        store_batches(batch_map(&[("batch-a", 64), ("batch-b", 64)]));
        assert_eq!(memo_batch_ids(), vec!["batch-a", "batch-b"]);
        store_batches(batch_map(&[("batch-a", 64)]));
        assert_eq!(memo_batch_ids(), vec!["batch-a"]);
        assert!(decoded("batch-b", &vec![b'x'; 64]).is_none());
    }

    /// #1394：Delta 域是缓存身份的一部分——同 session/catalog/profile 但模式不同不得复用已核前缀。
    #[test]
    fn storage_format_tag_is_part_of_the_verified_root_identity() {
        let with_format = |tag: &str| {
            (
                "test-c".to_string(),
                "catalog-hash".to_string(),
                ("profile-id".to_string(), "profile-hash".to_string()),
                tag.to_string(),
            )
        };
        configure(4096);
        store_root(
            rows(3),
            with_format(delta_storage::CHANGES_ONLY),
            empty_walk(),
        );
        assert_eq!(
            resume(
                &rows(3),
                &with_format(delta_storage::CHANGES_ONLY),
                &mut empty_walk()
            ),
            3
        );
        assert_eq!(
            resume(&rows(3), &with_format("legacy-full/0"), &mut empty_walk()),
            0
        );
        assert_eq!(last_resumed(), 0);
        configure(0);
    }
}
