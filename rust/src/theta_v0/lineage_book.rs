//! #679 D1b：`LineageBook`——重基构造谱系簿（旧中枢身份 → 新中枢身份的 1→1 映射）。
//!
//! ## 它解决什么
//!
//! 开放 frontier 每 bar 被 pop 后重扫，`CenterId=(start_index, zd, zg)` 是当次前三个构造单元的
//! 派生值：第三源外缘一改，父核心就重新求交，旧三元组消失、新三元组出现。生命周期层随后以新链
//! 全量 `adopt`，挂起簿只做 `CenterId` 精确集合核对，旧身份即被报作 `RebaseVanished`——这是
//! **工程丢身份**，不是教义生死（ADR 补充十三）。
//!
//! D1a（#543）已在塔构造层落下可离线复核的 [`RebaseTransformTxnV1`](super::classifier::rebase_txn)
//! 构造证书：每次重基事务带 old/new 完整四边框、**有序**源见证、递归链接与变换边
//! （`relation` + `functional/injective/unique/bijective`）。本模块**消费同一产出点**，在进程内
//! 把证书里 `relation == continued_1to1 ∧ 四布尔全真` 的边抽成 `旧身份 → 新身份` 的谱系簿，
//! 供接线层在无主核对前把挂起随谱系迁移。
//!
//! ## 教义口径（#679 票面，用户 2026-07-29 裁定链）
//!
//! - 过继 = **身份保持**（不丢），非复活（与 #292 F 裁定分清）；
//! - 只认构造层稳定边：一对一 / 唯一 / 函数性 / 单射 / 双射**全满足**才自动过继；
//!   分裂、真删除、歧义一律**拒绝**；
//! - **fail closed**：证书缺失 / bar 不匹配 / 目标不在新链上 / 同一旧身份有多个候选 ⟹
//!   保持现有 `RebaseVanished` 核销 + 工程错误观测计数，**禁 fail-open**；
//! - 首次绑定冻结的四边框不动：迁移只迁身份锚，清算仍按 ADR 补充十四拿旧框判。
//!
//! ## 与 `OPSEM_DUMP_DIR` 解耦（票面范围第 1 条）
//!
//! D1a 的证书**落盘**仍由 `OPSEM_DUMP_DIR` 门控；但**建簿**只看本模块的
//! [`consumer_enabled`]，默认开——生产判径可用，不只 dump 路径可用。反证开关
//! `THETA_REBASE_MIGRATE_SKIP=1` 把两者一并关回 D1a 前的旧行为。
//!
//! ## 两种读法（#679 用户 2026-07-29 裁定：宽读法转生产默认）
//!
//! D1a 实测：**严格读法**在 wf8 八条案例上给 5 条 `continued_1to1`；**宽读法**（只比原始种子
//! ordinal，= D0 探针口径）给 7 条。差在 seq=24/55——它们的第三个种子源在下一级发生了九段分裂
//! （`split`），即「同 ordinal 但不是同一个对象」。教义裁定：**下级分裂头子继承父种子**——
//! 头子唯一保住窗口起点与核心，身份延续无歧义，合 D0 §6.2 防歧义本意；题一归因表 24/55 两条
//! 按此判全部合法过继，逐条吻合。故**宽读法转生产默认**；严格读法留档不采用，作反事实对照，
//! 用 `THETA_REBASE_MIGRATE_STRICT=1` 显式切回（原 `THETA_REBASE_MIGRATE_WIDE` 开关随本次裁定
//! 撤销，反向重开为 `_STRICT`）。
//!
//! ## 作用域与线程模型
//!
//! 簿是 **thread_local**、**单 bar** 的：`classify_*` 在某 bar 产出的边只在同 bar 被接线层消费
//! （生产循环里 `classify_at(i)` 与 `step_center_oscillation_*(i, ..)` 同线程紧邻）。
//! [`record_edges`] 见到新 bar 即整簿重置，[`lookup`] 对 bar 不符的请求返回
//! [`Verdict::BarMismatch`]（fail closed，不做跨 bar 猜测）。多窗并行各线程独立成簿。

#[cfg(test)]
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use super::classifier::center_lifecycle::CenterId;
use super::strategy::center_oscillation_trade::{LineageLookup, LineageVerdict};

/// 反证开关：置 `1` ⟹ 建簿与迁移全关，回到 D1a 前的旧行为（锚重订常例必需）。
/// 字面量单源于 [`super::env_registry::THETA_REBASE_MIGRATE_SKIP`]（C2 收口）。
pub const SKIP_ENV: &str = super::env_registry::THETA_REBASE_MIGRATE_SKIP;
/// 读法开关（#679 用户 2026-07-29 裁定后反向重开）：置 `1` ⟹ 用**严格读法**（追下级 lineage）
/// 的边建簿留档对照；默认（未置）走**宽读法**（只比原始种子 ordinal）——生产默认。
/// 字面量单源于 [`super::env_registry::THETA_REBASE_MIGRATE_STRICT`]（C2 收口）。
pub const STRICT_ENV: &str = super::env_registry::THETA_REBASE_MIGRATE_STRICT;

// ─────────────────────────────────────────────────────────────────────────────
//  开关
// ─────────────────────────────────────────────────────────────────────────────

fn env_flag(name: &'static str, cell: &'static OnceLock<bool>) -> bool {
    *cell.get_or_init(|| std::env::var(name).map(|v| v == "1").unwrap_or(false))
}

#[cfg(test)]
thread_local! {
    /// 单测覆盖（`None` ⟹ 走 env 默认）。env 在测试进程里是全局的，不能逐测试改。
    static CONSUMER_OVERRIDE: Cell<Option<bool>> = const { Cell::new(None) };
    static WIDE_OVERRIDE: Cell<Option<bool>> = const { Cell::new(None) };
}

/// 谱系簿是否建（= 挂起迁移是否启用）。**默认真**——与 `OPSEM_DUMP_DIR` 解耦。
pub fn consumer_enabled() -> bool {
    #[cfg(test)]
    {
        if let Some(v) = CONSUMER_OVERRIDE.with(|c| c.get()) {
            return v;
        }
    }
    static SKIP: OnceLock<bool> = OnceLock::new();
    !env_flag(SKIP_ENV, &SKIP)
}

/// 逐次重基的迁移/拒迁明细是否打到 stderr（`THETA_LINEAGE_TRACE=1`）——纯诊断，
/// 不进任何产物文件，也不参与任何判据。验收时用它把读数落到具体的 `(bar, level, 身份)`。
pub fn trace_enabled() -> bool {
    static TRACE: OnceLock<bool> = OnceLock::new();
    env_flag(super::env_registry::THETA_LINEAGE_TRACE, &TRACE)
}

/// 是否用宽读法的边建簿（#679 裁定后默认真；`THETA_REBASE_MIGRATE_STRICT=1` 切回严格读法）。
pub fn wide_reading() -> bool {
    #[cfg(test)]
    {
        if let Some(v) = WIDE_OVERRIDE.with(|c| c.get()) {
            return v;
        }
    }
    static STRICT: OnceLock<bool> = OnceLock::new();
    !env_flag(STRICT_ENV, &STRICT)
}

/// 单测：覆盖 [`consumer_enabled`]（`None` 恢复 env 默认）。
#[cfg(test)]
pub fn test_set_consumer(v: Option<bool>) {
    CONSUMER_OVERRIDE.with(|c| c.set(v));
}

/// 单测：覆盖 [`wide_reading`]（`None` 恢复 env 默认）。
#[cfg(test)]
pub fn test_set_wide(v: Option<bool>) {
    WIDE_OVERRIDE.with(|c| c.set(v));
}

// ─────────────────────────────────────────────────────────────────────────────
//  簿本体
// ─────────────────────────────────────────────────────────────────────────────

/// 一条谱系边的见证（证书 digest 是它的可离线复核凭据）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Witness {
    pub new: CenterId,
    pub txn_id: u64,
    pub txn_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    /// 唯一后继。
    One(Witness),
    /// 同一旧身份在同 bar 同级被判到**多个不同**新身份——歧义，D0 明令拒绝自动过继。
    Conflict,
}

/// 查簿结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// 有唯一 1→1 构造证书边。
    Continued(Witness),
    /// 簿里没有这条旧身份（本 bar 本级没有为它产出可过继的边）。
    NoCert,
    /// 同一旧身份有多个候选后继。
    Ambiguous,
    /// 簿记的 bar 与请求 bar 不同（跨 bar 不做猜测）。
    BarMismatch,
}

#[derive(Debug, Default)]
struct Book {
    bar: Option<usize>,
    strict: BTreeMap<(usize, CenterId), Entry>,
    wide: BTreeMap<(usize, CenterId), Entry>,
}

impl Book {
    fn reset_to(&mut self, bar: usize) {
        self.bar = Some(bar);
        self.strict.clear();
        self.wide.clear();
    }
}

thread_local! {
    static BOOK: RefCell<Book> = RefCell::new(Book::default());
}

fn insert(map: &mut BTreeMap<(usize, CenterId), Entry>, level: usize, old: CenterId, w: Witness) {
    match map.entry((level, old)) {
        std::collections::btree_map::Entry::Vacant(v) => {
            v.insert(Entry::One(w));
        }
        std::collections::btree_map::Entry::Occupied(mut o) => {
            // 同 bar 同级同旧身份被重复登记：同一后继 ⟹ 幂等无害（同 bar 内 classifier 可能
            // 被重放）；不同后继 ⟹ 歧义，永久置 Conflict（拒绝过继，fail closed）。
            let same = matches!(o.get(), Entry::One(prev) if prev.new == w.new);
            if !same {
                o.insert(Entry::Conflict);
            }
        }
    }
}

/// 登记一次重基事务的 1→1 边。由构造证书产出点（`rebase_txn::emit`）调用。
///
/// - `strict`：严格读法（追下级 lineage）的 `continued_1to1 ∧ 四布尔全真` 边；
/// - `wide`：宽读法（只比原始种子 ordinal）的同类边——只在 [`wide_reading`] 时非空。
///
/// `old == new` 的边不登记（无需迁移，登记只会污染簿）。
pub fn record_edges(
    bar: usize,
    level: usize,
    txn_id: u64,
    txn_digest: &str,
    strict: &[(CenterId, CenterId)],
    wide: &[(CenterId, CenterId)],
) {
    if strict.is_empty() && wide.is_empty() {
        // 仍要推进 bar：空事务也是「本 bar 没有可过继边」的事实，跨 bar 残留必须清掉。
        BOOK.with(|b| {
            let mut b = b.borrow_mut();
            if b.bar != Some(bar) {
                b.reset_to(bar);
            }
        });
        return;
    }
    BOOK.with(|b| {
        let mut b = b.borrow_mut();
        if b.bar != Some(bar) {
            b.reset_to(bar);
        }
        for &(old, new) in strict {
            if old != new {
                insert(&mut b.strict, level, old, Witness { new, txn_id, txn_digest: txn_digest.to_string() });
            }
        }
        for &(old, new) in wide {
            if old != new {
                insert(&mut b.wide, level, old, Witness { new, txn_id, txn_digest: txn_digest.to_string() });
            }
        }
    });
}

/// 查一条旧身份在本 bar 本级的谱系后继。
pub fn lookup(bar: usize, level: usize, old: CenterId) -> Verdict {
    BOOK.with(|b| {
        let b = b.borrow();
        if b.bar != Some(bar) {
            return Verdict::BarMismatch;
        }
        let map = if wide_reading() { &b.wide } else { &b.strict };
        match map.get(&(level, old)) {
            None => Verdict::NoCert,
            Some(Entry::Conflict) => Verdict::Ambiguous,
            Some(Entry::One(w)) => Verdict::Continued(w.clone()),
        }
    })
}

/// 测试/诊断：清空当前线程的簿。
pub fn reset_book() {
    BOOK.with(|b| *b.borrow_mut() = Book::default());
}

/// 测试/诊断：当前簿的边数 `(strict, wide)`。
pub fn book_len() -> (usize, usize) {
    BOOK.with(|b| {
        let b = b.borrow();
        (b.strict.len(), b.wide.len())
    })
}

// ─────────────────────────────────────────────────────────────────────────────
//  接线层视图（域层只见 trait，不依赖本模块的具体类型）
// ─────────────────────────────────────────────────────────────────────────────

/// 绑定到某个 `(bar, level)` 的查簿视图——交给挂起簿做谱系迁移判定。
#[derive(Debug, Clone, Copy)]
pub struct LineageView {
    pub bar: usize,
    pub level: usize,
}

impl LineageLookup for LineageView {
    fn lookup(&self, old: CenterId) -> LineageVerdict {
        match lookup(self.bar, self.level, old) {
            Verdict::Continued(w) => LineageVerdict::Continued(w.new),
            Verdict::NoCert => LineageVerdict::NoCert,
            Verdict::Ambiguous => LineageVerdict::Ambiguous,
            Verdict::BarMismatch => LineageVerdict::BarMismatch,
        }
    }
}

/// 生产接线入口：迁移启用 ⟹ `Some(view)`；`THETA_REBASE_MIGRATE_SKIP=1` ⟹ `None`（旧行为）。
pub fn view_for(bar: usize, level: usize) -> Option<LineageView> {
    consumer_enabled().then_some(LineageView { bar, level })
}

// ─────────────────────────────────────────────────────────────────────────────
//  进程级计数（票面「迁移计数分桶」）
// ─────────────────────────────────────────────────────────────────────────────

macro_rules! counters {
    ($($(#[$m:meta])* $field:ident => $stat:ident),+ $(,)?) => {
        $(static $stat: AtomicU64 = AtomicU64::new(0);)+

        /// 迁移/拒迁计数快照。
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
        pub struct Counters {
            $($(#[$m])* pub $field: u64,)+
        }

        /// 读全部计数。
        pub fn counters() -> Counters {
            Counters { $($field: $stat.load(Ordering::Relaxed),)+ }
        }

        /// 清零全部计数（测试隔离用）。
        pub fn reset_counters() {
            $($stat.store(0, Ordering::Relaxed);)+
        }
    };
}

counters! {
    /// 挂起随谱系迁移成功的次数（`rebase_lineage_migrated`）。
    migrated => MIGRATED,
    /// 谱系不可达、保持 `RebaseVanished` 核销的次数（`rebase_vanished_kept`）。
    kept => KEPT,
    /// fail closed 分桶：本 bar 本级无该旧身份的构造证书边。
    fail_no_cert => FAIL_NO_CERT,
    /// fail closed 分桶：证书给出的新身份不在重基后的新链上。
    fail_target_absent => FAIL_TARGET_ABSENT,
    /// fail closed 分桶：同一旧身份有多个候选后继（歧义）。
    fail_ambiguous => FAIL_AMBIGUOUS,
    /// fail closed 分桶：簿记 bar 与请求 bar 不符（工程错误，不做跨 bar 猜测）。
    fail_bar_mismatch => FAIL_BAR_MISMATCH,
    /// fail closed 分桶：多个旧锚被判到同一新身份（重复 lineage claim）。
    fail_duplicate_claim => FAIL_DUP_CLAIM,
}

/// 累加一次重基的迁移分桶读数（接线层在 `on_chain_rebase_lineage` 之后调用）。
pub fn record_tally(t: &super::strategy::center_oscillation_trade::RebaseMigrationTally) {
    MIGRATED.fetch_add(t.migrated as u64, Ordering::Relaxed);
    KEPT.fetch_add(t.kept as u64, Ordering::Relaxed);
    FAIL_NO_CERT.fetch_add(t.no_cert as u64, Ordering::Relaxed);
    FAIL_TARGET_ABSENT.fetch_add(t.target_absent as u64, Ordering::Relaxed);
    FAIL_AMBIGUOUS.fetch_add(t.ambiguous as u64, Ordering::Relaxed);
    FAIL_BAR_MISMATCH.fetch_add(t.bar_mismatch as u64, Ordering::Relaxed);
    FAIL_DUP_CLAIM.fetch_add(t.duplicate_claim as u64, Ordering::Relaxed);
}

/// 一行读数（验收跑收尾打印，便于从 log 直接取数）。
pub fn report_line() -> String {
    let c = counters();
    format!(
        "[LINEAGE] reading={} migrated={} vanished_kept={} \
         fail(no_cert={} target_absent={} ambiguous={} bar_mismatch={} dup_claim={})",
        if wide_reading() { "wide" } else { "strict" },
        c.migrated,
        c.kept,
        c.fail_no_cert,
        c.fail_target_absent,
        c.fail_ambiguous,
        c.fail_bar_mismatch,
        c.fail_duplicate_claim,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(start: usize, zd: i64, zg: i64) -> CenterId {
        CenterId { start_index: start, zd, zg }
    }

    /// 同 bar 登记的唯一 1→1 边可查出，且带证书 digest 见证。
    #[test]
    fn records_and_looks_up_unique_edge() {
        reset_book();
        test_set_wide(Some(false));
        let (old, new) = (cid(10, 100, 200), cid(10, 90, 200));
        record_edges(7, 1, 42, "deadbeef", &[(old, new)], &[]);
        match lookup(7, 1, old) {
            Verdict::Continued(w) => {
                assert_eq!(w.new, new);
                assert_eq!(w.txn_id, 42);
                assert_eq!(w.txn_digest, "deadbeef");
            }
            other => panic!("应查到唯一后继，实得 {other:?}"),
        }
        test_set_wide(None);
    }

    /// fail closed 三族：跨 bar / 无边 / 歧义。
    #[test]
    fn fails_closed_on_bar_mismatch_missing_and_conflict() {
        reset_book();
        test_set_wide(Some(false));
        let old = cid(10, 100, 200);
        record_edges(7, 1, 1, "d", &[(old, cid(10, 90, 200))], &[]);
        assert_eq!(lookup(8, 1, old), Verdict::BarMismatch, "跨 bar 不做猜测");
        assert_eq!(lookup(7, 1, cid(99, 1, 2)), Verdict::NoCert, "簿里没有 ⟹ 无证书");
        assert_eq!(lookup(7, 2, old), Verdict::NoCert, "级别不符 ⟹ 无证书");
        // 同 bar 同级同旧身份被判到第二个后继 ⟹ 歧义。
        record_edges(7, 1, 2, "d2", &[(old, cid(10, 80, 200))], &[]);
        assert_eq!(lookup(7, 1, old), Verdict::Ambiguous);
        test_set_wide(None);
    }

    /// 新 bar 到达 ⟹ 整簿重置（含「空事务也推进 bar」）。
    #[test]
    fn new_bar_resets_book_even_when_edge_set_is_empty() {
        reset_book();
        test_set_wide(Some(false));
        let old = cid(10, 100, 200);
        record_edges(7, 1, 1, "d", &[(old, cid(10, 90, 200))], &[]);
        assert_eq!(book_len().0, 1);
        record_edges(8, 1, 2, "d", &[], &[]);
        assert_eq!(book_len().0, 0, "空事务也须把上一 bar 的残留清掉");
        assert_eq!(lookup(8, 1, old), Verdict::NoCert);
        test_set_wide(None);
    }

    /// 恒等边（old == new）不入簿——它不需要迁移。
    #[test]
    fn identity_edge_is_not_recorded() {
        reset_book();
        test_set_wide(Some(false));
        let same = cid(10, 100, 200);
        record_edges(7, 1, 1, "d", &[(same, same)], &[]);
        assert_eq!(book_len().0, 0);
        test_set_wide(None);
    }

    /// ★#679 用户 2026-07-29 裁定守卫：未置任何环境变量（无覆盖）⟹ `wide_reading()` 默认真
    /// ——防裁定被静默改回严格默认。`THETA_REBASE_MIGRATE_STRICT` 未设时才有意义，故先断言。
    #[test]
    fn wide_reading_defaults_true_without_any_env_override() {
        assert!(std::env::var(STRICT_ENV).is_err(), "本测试要求进程未设 {STRICT_ENV}");
        test_set_wide(None);
        assert!(wide_reading(), "#679 裁定：宽读法转生产默认，未置开关时须为真");
    }

    /// 严格/宽两读法各自成簿，互不串味。
    #[test]
    fn strict_and_wide_books_are_separate() {
        reset_book();
        let old = cid(10, 100, 200);
        let strict_new = cid(10, 90, 200);
        let wide_new = cid(10, 80, 200);
        record_edges(7, 1, 1, "d", &[(old, strict_new)], &[(old, wide_new)]);
        test_set_wide(Some(false));
        assert!(matches!(lookup(7, 1, old), Verdict::Continued(w) if w.new == strict_new));
        test_set_wide(Some(true));
        assert!(matches!(lookup(7, 1, old), Verdict::Continued(w) if w.new == wide_new));
        test_set_wide(None);
    }
}
