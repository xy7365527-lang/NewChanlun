//! Lean ↔ Rust golden vector 交叉验证——#246「相切=重合」裁定的 **bit-exact 机器见证**。
//!
//! ## 并线留痕（#614，2026-07-29）
//!
//! 本文件原含 §1–§6 卖侧（`Origin.SellPointRecog`/`SellClosedLoop`）parity 段。main 线
//! commit `aa75566a67`（#181「先迁消费方后删路径」裁定）已把验证对象
//! `theta_v0/closed_loop/sell.rs`（432 行）整体退役、`pub mod sell` 卸载，并同批整删本文件
//! （398 行，理由原文：「验证对象整体退役」）。kimi 线同期在本文件新增 §7 相切机器见证
//! （#248 落码 → #296 Decidable → #312 主缝锁 → #319 端点机器耦合），其依赖只有
//! `parser::segment::Interval`，与已退役的卖侧无关。
//!
//! 并线解法 = **并集**：卖侧段随其验证对象退役（取 main），§7 逐字保留（取 kimi）。
//! ⚠ 待人工复核：本文件属证书真值路径，§7 以下内容与 kimi tip `797c9ad35c` 逐字相同，
//! 未作任何语义改写；被删的仅是引用已不存在符号的卖侧段。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! **L0**（Lean machine-checked 见证 ↔ rust 实装对齐）。绿 = rust `Interval::gap`/`overlaps`
//! 对 Lean `decide(HasGap)`/`decide(Overlaps)` 的机器导出值逐用例 bit-exact。
//! 这**不**提升经验等级——不是缠论盈利/实盘有效声明。

use newchan_rust::theta_v0::parser::segment::Interval;
use serde::Deserialize;

#[derive(Deserialize)]
struct ParityFixture {
    gap_overlap: GapOverlapSection,
}

/// #246 相切=重合裁定（ticket #248）fixture 段：单区间对的 Lean 机器见证
/// （`decide (HasGap ..)` / `decide (Overlaps ..)` 真求值——Overlaps 已补 Decidable
/// instance（#296，SegmentFeatureSeq.lean:114，与 :105 HasGap 同范式）直接求值；
/// 历史：#248 落地时无 instance，曾经已证 `gap_iff_not_overlap`（:118）严格互推绕道）。
#[derive(Deserialize)]
struct GapOverlapSection {
    tangent_a_high_eq_b_low: GapOverlapCase,
    tangent_b_high_eq_a_low: GapOverlapCase,
    strict_disjoint: GapOverlapCase,
    strict_disjoint_rev: GapOverlapCase,
    strict_overlap: GapOverlapCase,
}

/// 一条用例：四个输入端点 + 两个真值，**全部**由 Lean 机器导出（端点自 #319 起入 fixture，
/// 此前是本文件与 `parser::gap_overlap_fixture` 各誊写一份的人工对照）。
#[derive(Deserialize)]
struct GapOverlapCase {
    a_low: i64,
    a_high: i64,
    b_low: i64,
    b_high: i64,
    has_gap: bool,
    overlaps: bool,
}

impl GapOverlapCase {
    /// fixture 端点 → rust `Interval` 对（`lo`/`hi` 直取导出值，禁手填）。
    fn intervals(&self) -> (Interval, Interval) {
        (
            Interval {
                lo: self.a_low,
                hi: self.a_high,
            },
            Interval {
                lo: self.b_low,
                hi: self.b_high,
            },
        )
    }
}

fn load_fixture() -> ParityFixture {
    let raw = include_str!("fixtures/theta_v0_parity.json");
    serde_json::from_str(raw).expect("fixture 必须是 Lean #eval 导出的合法 JSON")
}

// ════════════════════════════════════════════════════════════════════════════
//  §7 #246 相切=重合裁定机器见证（ticket #248，2026-07-25 裁定书
//     chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md）
//
//  裁定：相切（两区间只有一个公共端点）算「有重合区间」⟹ 无缺口，全域生效。
//  Lean 侧 `HasGap`（严格 `<`，SegmentFeatureSeq.lean:102）与 `Overlaps`（`≤`，:112）
//  本已符合裁定且 `gap_iff_not_overlap`（:118）已证互补；rust 侧 `Interval::overlaps`（≤）
//  与 `Interval::gap`（!overlaps）与之逐字对齐。本测试断言：对同一批区间对用例，
//  rust 既有原语计算 == fixture 中 Lean `decide` 机器导出值。
//
//  用例区间端点与期望值（has_gap/overlaps）**全部**从 fixture 读（Lean #eval 机器产，禁手填）
//  ——端点自 #319（#316 影子评审 MED-1）起由 `gapOverlapJson` 读 `FeatureElem` 字段导出，
//  本文件不再誊写。
//  Overlaps 真值经 `decide (Overlaps ..)` 直接读出（#296 已补 Decidable instance，
//  SegmentFeatureSeq.lean:114；历史：#248 时经 `decide (¬ HasGap ..)` 由
//  `gap_iff_not_overlap`（:118）互推，090 留痕）。
//  三笔形态（max(lows)==min(highs)）不适用：Lean 无三笔重合谓词，其相切语义与两区间
//  形态同构。
//
//  ⚠有效域（#276 影子评审 HIGH-1 → 票 #312，090 诚实边界）：本测试断言的 `Interval::gap` /
//  `Interval::overlaps` 是**第三份实现**，#246 裁定并未改动它。真正被改口径的两个生产谓词
//  （`parser::feature_seq::is_fractal_and_gap` / `parser::segment::three_stroke_overlap`）是私有
//  fn，crate 外调不到，**不在本测试覆盖内**——它们的机器耦合缝在 crate 内：
//  `parser::gap_overlap_fixture`（读同一份 fixture）+ 两模块的
//  `*_lean_fixture_bit_exact` 单测（`cargo test --lib`）。本测试绿 ≠ 那两个谓词的口径已锁。
//
//  ★端点转录漂移已消（#319）：端点唯一权威源是导出器 `:204-208` 的 `feOf` 参数，经
//  `gapOverlapJson` 机器导出进 fixture；本文件与 `parser::gap_overlap_fixture::gap_overlap_cases`
//  都改读 fixture 端点，rust 侧零誊写（此前两处各誊写一份，任一处漏改都不会红——此风险已消）。
//  ⚠诚实口径（同 `gap_overlap_fixture.rs:141-145`，090）：Lean 端点改不等于本测试必红——
//  端点变动若改了用例形态或真值，`fixture_endpoints_match_kind` 与两个消费方谓词单测才会红；
//  若端点变动既不改形态也不改真值（如 `strict_disjoint` 的 `b_low` 11→12），rust 侧**不红也
//  不该红**——两侧同源于同一份 fixture，此时不存在不一致，红了反而是假阳性。
// ════════════════════════════════════════════════════════════════════════════

/// rust `Interval::overlaps`/`gap` == Lean `decide(HasGap)`/`decide(¬HasGap)` 逐用例 bit-exact。
#[test]
fn lean_gap_overlap_tangent_bit_exact() {
    let fx = load_fixture();
    // (Lean 机器导出用例, 用例名)——区间端点与真值同源于 fixture 的 gap_overlap 段（#319）。
    let cases: [(&GapOverlapCase, &str); 5] = [
        (
            &fx.gap_overlap.tangent_a_high_eq_b_low,
            "tangent_a_high_eq_b_low（a.high == b.low 相切）",
        ),
        (
            &fx.gap_overlap.tangent_b_high_eq_a_low,
            "tangent_b_high_eq_a_low（反向相切）",
        ),
        (
            &fx.gap_overlap.strict_disjoint,
            "strict_disjoint（严格分离）",
        ),
        (
            &fx.gap_overlap.strict_disjoint_rev,
            "strict_disjoint_rev（反向严格分离）",
        ),
        (
            &fx.gap_overlap.strict_overlap,
            "strict_overlap（严格重叠对照）",
        ),
    ];
    for (expected, name) in cases {
        // 端点侧不变量（`FeatureElem.valid`：low ≤ high）不在本文件重复断言——
        // crate 内 `parser::gap_overlap_fixture::tests::fixture_endpoints_are_valid_intervals`
        // 已守同一条不变量（同一份 fixture）；此处接反端点仍会被下面的谓词失配捕获——
        // 但只对 5 例中的 3 例（两条 tangent + strict_overlap）成立：strict_disjoint /
        // strict_disjoint_rev 一对互为接反对照，接反后真值不变，不会触发失配。此守卫是
        // **集合级**（5 例作为一组，覆盖了接反会翻真值的情形）而非逐用例级，如实登记。
        let (a, b) = expected.intervals();
        assert_eq!(
            a.gap(&b),
            expected.has_gap,
            "{name}: rust Interval::gap == Lean decide(HasGap)（bit-exact）"
        );
        assert_eq!(
            a.overlaps(&b),
            expected.overlaps,
            "{name}: rust Interval::overlaps == Lean decide(¬HasGap)（经 gap_iff_not_overlap ↔ Overlaps）"
        );
        // 互补性在 Bool 层自证：fixture 导出值自身须满足 has_gap == !overlaps。
        assert_eq!(
            expected.has_gap, !expected.overlaps,
            "{name}: fixture 导出值违反 gap_iff_not_overlap 互补（Lean 侧漂移）"
        );
    }
}
