//! #246 相切=重合口径的 **crate 内机器耦合缝**（ticket #312，兑现 #248 reopen 的 HIGH-1）。
//!
//! ## 立项事实（#276 影子评审 HIGH-1，`chanlun/review-results/shadow-review-248-20260726.md`）
//!
//! #248 落的主缝 `rust/tests/theta_v0_lean_parity.rs::lean_gap_overlap_tangent_bit_exact` 断言的是
//! `Interval::gap` / `Interval::overlaps`（`segment.rs:90-97`）——那是**本次未改动**的第三份实现。
//! 真正被 #246 裁定改口径的两个谓词 `feature_seq::is_fractal_and_gap`（缺口，严格 `>`）与
//! `segment::three_stroke_overlap`（三笔重合，含等号 `<=`）是**私有 fn**，crate 外集成测试调不到：
//! 把它们改回旧口径，parity 依旧 11/11 全绿。即主缝对本次统一的口径**零覆盖**。
//!
//! ## 本模块的位置
//!
//! 修法取票 #312 的 (b)：**不扩谓词可见性**，把两模块内既有的手填期望值单测改读**同一份** Lean
//! 机器导出 fixture（`rust/tests/fixtures/theta_v0_parity.json` 的 `gap_overlap` 段，导出器
//! `formal/Origin/ParityFixtureExport.lean:204-208`）。本模块是这条耦合链的唯一读取点，供
//! `feature_seq::tests` 与 `segment::tests` 共用（避免两处重复 JSON 解析）。
//!
//! ## 手填 vs 机器耦合的分界（090 诚实边界，逐项登记）
//!
//! - **两谓词单测的期望值**（`has_gap` / `overlaps`）：全部从 fixture 反序列化，**禁手填**——
//!   Lean `decide` 求值产物，是本模块存在的理由；
//! - **用例区间端点**（`a` / `b`）：自 **#319**（#316 影子评审 MED-1）起同样从 fixture 读
//!   （导出器 `gapOverlapJson` 增导 `a_low`/`a_high`/`b_low`/`b_high`，读 `FeatureElem` 字段求值）。
//!   此前是逐字誊写导出器 `feOf` 参数的人工对照，且同一组端点存在于本文件与主缝
//!   `theta_v0_lean_parity.rs` §7 两处，Lean 端点改而 rust 漏改**两侧都不会红**；现端点唯一权威源
//!   在导出器，rust 侧零誊写，漂移沿「Lean 改端点 ⟹ fixture 变 ⟹ 用例输入变 ⟹ 谓词结果与导出真值
//!   不符」即红；
//! - **本模块自身的 `tests`**：`GapOverlapKind` 与 fixture 真值的对应关系是 **#246 裁定口径的人工
//!   编码**（相切⟹无缺口 / 严格分离⟹有缺口），不是机器导出。裁定若翻转，该守卫须同步改。
//!   `kind` 本身仍是人工归类，但其与端点的相符性由 `tests::fixture_endpoints_match_kind` 核实。
//!
//! ## 有效域（自动化层缺一腿，如实登记）
//!
//! 本缝只在 `cargo test --lib` 下生效。仓内 CI（`.github/workflows/ci.yml`）当前只跑 pytest 与
//! `fixture-drift`，**不跑 `cargo test`**——故「口径回改即红」在 CI 自动化层仍未闭合（#312 未要求
//! CI 腿，此处登记边界，不冒充已闭合）。
//!
//! ## 认识论等级（`formalization-validity-domain.md` 231号）
//!
//! **L0**（纯定义 / 机器求值，不依赖任何市场数据——Lean `decide` 对形式化定义求值 ↔ rust 谓词对
//! 同一组构造输入求值）。绿 = 两个生产谓词的相切行为忠实于形式化定义，**不**构成任何盈利 / 实盘
//! 有效声明（那需要 L2/L3 真实数据证据，本缝不提供）。

use super::super::types::Tick;

/// Lean 侧一条用例的机器导出：四个输入端点（`FeatureElem` 字段求值）+ 两个真值
/// （`decide (HasGap a b)` / `decide (Overlaps a b)`）。端点自 #319 起随 fixture 导出，
/// rust 侧不再誊写。
#[derive(serde::Deserialize, Clone, Copy)]
struct LeanCase {
    a_low: Tick,
    a_high: Tick,
    b_low: Tick,
    b_high: Tick,
    has_gap: bool,
    overlaps: bool,
}

#[derive(serde::Deserialize)]
struct GapOverlapExport {
    tangent_a_high_eq_b_low: LeanCase,
    tangent_b_high_eq_a_low: LeanCase,
    strict_disjoint: LeanCase,
    strict_disjoint_rev: LeanCase,
    strict_overlap: LeanCase,
}

#[derive(serde::Deserialize)]
struct FixtureRoot {
    gap_overlap: GapOverlapExport,
}

/// 用例的几何形态（穷举，新增用例必须显式归类——不按名字前缀猜）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum GapOverlapKind {
    /// 两区间只共一个端点（`upper.lo == lower.hi`）。
    Tangent,
    /// 两区间严格分离（`upper.lo > lower.hi`）。
    StrictDisjoint,
    /// 两区间严格重叠（`upper.lo < lower.hi`）。
    StrictOverlap,
}

/// 一条 Lean 导出用例：机器导出端点 + 机器导出真值（两者同源，#319 后 rust 侧零手填）。
#[derive(Clone, Copy)]
pub(super) struct GapOverlapCase {
    /// 用例名（与 fixture / 导出器同名，失败信息里指认到具体形态）。
    pub name: &'static str,
    /// 几何形态（人工归类，由 `tests::fixture_endpoints_match_kind` 对端点核实）。
    pub kind: GapOverlapKind,
    /// 区间 a `[lo, hi]`（fixture 读出 `a_low`/`a_high`，禁手填）。
    pub a: (Tick, Tick),
    /// 区间 b `[lo, hi]`（fixture 读出 `b_low`/`b_high`，禁手填）。
    pub b: (Tick, Tick),
    /// Lean `decide (HasGap a b)`（fixture 读出，禁手填）。
    pub has_gap: bool,
    /// Lean `decide (Overlaps a b)`（fixture 读出，禁手填）。
    pub overlaps: bool,
}

impl GapOverlapCase {
    /// 按 `lo` 升序定序为（下方元素, 上方元素）。
    ///
    /// 两个生产谓词是**方向化**的（`is_fractal_and_gap` 向上段看 `b_l > a_h`、向下段看 `a_l > b_h`），
    /// 而 Lean `HasGap` / `Overlaps` 对称（fixture 的 `*_rev` 用例与正向用例真值相同即其见证）。
    /// 定序把对称谓词的真值接到方向化谓词的两个分支上，两分支期望值同为 `has_gap`。
    ///
    /// ★副作用（如实登记）：定序后 `*_rev` 两条用例与其正向用例的输入**逐字相同**——rust 侧
    /// 实得 3 组互异输入（相切 / 严格分离 / 严格重叠），rev 两条是 Lean 对称性的冗余确认，不产生
    /// 新的 rust 输入。「相切的两个方向」由消费方各自跑 Up / Down 两分支取得，不靠 rev 用例。
    pub(super) fn ordered(&self) -> ((Tick, Tick), (Tick, Tick)) {
        if self.a.0 <= self.b.0 {
            (self.a, self.b)
        } else {
            (self.b, self.a)
        }
    }
}

/// 把 fixture 的一条导出用例接成 `GapOverlapCase`（端点与真值同源，唯一人工输入是 `name`/`kind`）。
fn from_lean(name: &'static str, kind: GapOverlapKind, l: LeanCase) -> GapOverlapCase {
    GapOverlapCase {
        name,
        kind,
        a: (l.a_low, l.a_high),
        b: (l.b_low, l.b_high),
        has_gap: l.has_gap,
        overlaps: l.overlaps,
    }
}

/// 读 fixture，返回 5 条 Lean 导出用例（相切正/反向 + 严格分离正/反向 + 严格重叠对照）。
///
/// 端点与真值**全部**来自 fixture（#319）；用例名不含具体端点数字——端点已非本文件所有，写进名字
/// 就是重新引入手抄面。失败信息需要端点时由消费方打印 `case.a` / `case.b`。
///
/// fixture 路径与主缝 `theta_v0_lean_parity.rs:154` 指向**同一个文件**（`include_str!` 编译期内联，
/// 文件被 `scripts/check_fixture_drift.py` / CI `fixture-drift` job 守护，漂移即红）。
///
/// ★「端点不符即红」沿哪段传（#319 诚实登记，090）：这条链是**两段**接起来的——
/// Lean 源 ↔ fixture 一段由 drift gate 守（Lean 改端点不再生 fixture ⟹ `check_fixture_drift.py` 红）；
/// fixture ↔ rust 一段由本文件的单源读取消解（rust 无第二份端点，结构上不存在「漏改」）。
/// 再生后端点若改变了用例形态或真值，`fixture_endpoints_match_kind` 与两个消费方谓词单测即红；
/// 若端点变动既不改形态也不改真值（如 `strict_disjoint` 的 `b_low` 11→12），rust 侧**不红也不该红**
/// ——两侧同源，此时不存在不一致。
pub(super) fn gap_overlap_cases() -> [GapOverlapCase; 5] {
    let raw = include_str!("../../../tests/fixtures/theta_v0_parity.json");
    let fx: FixtureRoot =
        serde_json::from_str(raw).expect("fixture 必须是 Lean #eval 导出的合法 JSON");
    let g = fx.gap_overlap;
    [
        from_lean(
            "tangent_a_high_eq_b_low（a.high == b.low 相切）",
            GapOverlapKind::Tangent,
            g.tangent_a_high_eq_b_low,
        ),
        from_lean(
            "tangent_b_high_eq_a_low（反向相切）",
            GapOverlapKind::Tangent,
            g.tangent_b_high_eq_a_low,
        ),
        from_lean(
            "strict_disjoint（严格分离）",
            GapOverlapKind::StrictDisjoint,
            g.strict_disjoint,
        ),
        from_lean(
            "strict_disjoint_rev（反向严格分离）",
            GapOverlapKind::StrictDisjoint,
            g.strict_disjoint_rev,
        ),
        from_lean(
            "strict_overlap（严格重叠对照）",
            GapOverlapKind::StrictOverlap,
            g.strict_overlap,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// fixture 端点自身须满足 `FeatureElem.valid`（`low ≤ high`）——Lean 侧类型不变量的导出侧确认。
    /// 防的是导出器把 low/high 接反（`a_low`/`a_high` 字段写串），这类漂移不改真值、只改端点。
    #[test]
    fn fixture_endpoints_are_valid_intervals() {
        for c in gap_overlap_cases() {
            assert!(
                c.a.0 <= c.a.1 && c.b.0 <= c.b.1,
                "{}: fixture 端点违反 FeatureElem.valid（low ≤ high）：a={:?} b={:?}",
                c.name,
                c.a,
                c.b
            );
        }
    }

    /// fixture 自身须满足 Lean **已证定理** `gap_iff_not_overlap`（`HasGap ↔ ¬Overlaps`）。
    /// 防的是 Lean 侧定义漂移（导出值互补性破裂），与两个 rust 谓词的实装无关；判据来自已证
    /// 定理，非人工口径编码。
    #[test]
    fn fixture_truths_are_complementary() {
        for c in gap_overlap_cases() {
            assert_eq!(
                c.has_gap, !c.overlaps,
                "{}: fixture 导出值违反 gap_iff_not_overlap 互补（Lean 侧漂移）",
                c.name
            );
        }
    }

    /// fixture 端点 ↔ `kind` ↔ fixture 真值 三者自洽。
    ///
    /// #319 后端点与真值同源于 fixture，本测试防的是**人工归类的 `kind` 与机器端点脱节**——
    /// Lean 侧把某用例端点改成另一形态（如 `strict_disjoint` 改到相切）而此处 `kind` 未跟，即红。
    /// ⚠本测试的「形态 → 真值」映射是 **#246 裁定口径的人工编码**（相切⟹无缺口），不是机器导出：
    /// 裁定书 `chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md` §1。若裁定按其
    /// §3.1 已登记的重议触发条件翻转，**本测试须同步改**——届时它红的含义是「裁定已变」，不是
    /// 「Lean 漂移」。
    #[test]
    fn fixture_endpoints_match_kind() {
        for c in gap_overlap_cases() {
            let (lower, upper) = c.ordered();
            let geometric = if upper.0 == lower.1 {
                GapOverlapKind::Tangent
            } else if upper.0 > lower.1 {
                GapOverlapKind::StrictDisjoint
            } else {
                GapOverlapKind::StrictOverlap
            };
            assert_eq!(
                geometric, c.kind,
                "{}: fixture 端点的几何形态与登记的 kind 不符（Lean 侧端点已改，此处 kind 未跟）",
                c.name
            );
            let expected_gap = c.kind == GapOverlapKind::StrictDisjoint;
            assert_eq!(
                c.has_gap, expected_gap,
                "{}: fixture 真值与 #246 裁定口径不符（裁定翻转 ⟹ 本守卫须同步改；否则 Lean 漂移）",
                c.name
            );
        }
    }
}
