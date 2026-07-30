//! GUARD-ROLE: toplevel-loose-files-adjunct——名分：**测试基础设施现役**（非五态自动归批，
//! #764 C7-E5 单列核定，按实测判；不随 `lib.rs` 头部 GUARD-ROLE 块「现役」批标，亦不进批
//! 内 14 文件枚举）。实测：全仓零非测试调用者（`#[path]` 挂 `segment.rs:824` 仅测试内
//! `mod tests`），但本文件 6 个 `#[test]` 在 `cargo test --lib`（常规跑，非 `--ignored`）下
//! 全部 `ok`（`segment::tests::is_fractal_and_gap_*` 等，2026-07-30 复核实测）——是 #317
//! 相切口径回归锁的活跃测试主体，非死代码。沿 #763 trading/ 三个 `#[cfg(test)]` 消融文件
//! 先例（`consolidation_ablation.rs` 等）：「零活引用⟹删除」判据前提是生产/python 可达性，
//! 不适用测试基础设施存废判定。处置：留档零删除，零移入 `legacy/`。
//!
//! 相切口径回归锁（#317；#288 影子评审 HIGH-1，
//! chanlun/review-results/shadow-review-288-20260726.md §三）
//! #246 相切=重合口径在 **legacy 段引擎两个私有谓词**上的回归锁。
//!
//! ## 立项事实（#302 评审 HIGH-1）
//!
//! #288 把 `three_stroke_overlap` 切为含端点 `<=`、`is_fractal_and_gap` 缺口臂切为严格 `>`，
//! 但**零新增单测**。两谓词是私有 fn，crate 外集成测试调不到；唯一能覆盖它们的
//! `tests/test_rust_segment_equivalence.py::test_segment_bitexact_real` 依赖 gitignored 的
//! `.cache/BZ_1min_2024_raw.parquet`，在 CI/新克隆环境 `pytest.skip` ⟹ 把谓词改回旧口径，
//! 全套非 slow 仍全绿。本模块补上这条锁：口径回改，下列相切用例立刻红并指认到具体谓词。
//!
//! ## 手填 vs 机器耦合（090 诚实边界）
//!
//! 本模块用例的期望值是 **#246 裁定口径的人工编码**（相切 ⟹ 有重合 / 相切 ⟹ 无缺口），
//! **不是** Lean 机器导出——theta_v0 侧的机器耦合缝（`theta_v0/parser/gap_overlap_fixture.rs`，
//! #312）读 `rust/tests/fixtures/theta_v0_parity.json`，其模块声明为 `theta_v0::parser` 私有，
//! legacy 侧不可达；本票不扩其可见性（不在 #317 工作面）。故本模块与 #248 的
//! `three_stroke_overlap_distinct_triple_tangent` 同性质：裁定若翻转，这里须同步改。
//! 如实登记，不冒充机器耦合。
//!
//! ## 认识论等级（`formalization-validity-domain.md` 231号）
//!
//! **L0**（纯定义/构造输入求值，不依赖任何市场数据）。绿 = 两谓词的相切行为忠实于 #246 裁定，
//! **不**构成任何实盘/盈利有效声明。

use super::*;

/// 构造一条只用于谓词求值的笔（`three_stroke_overlap` 只读 `high`/`low`）。
fn stroke_hl(direction: Direction, i0: usize, i1: usize, high: f64, low: f64) -> Stroke {
    let (p0, p1) = match direction {
        Direction::Up => (low, high),
        Direction::Down => (high, low),
    };
    Stroke {
        i0,
        i1,
        direction,
        high,
        low,
        p0,
        p1,
        confirmed: true,
    }
}

#[test]
fn three_stroke_overlap_distinct_triple_tangent() {
    // 三笔互异区间 [5,10] / [8,12] / [10,15]：max(lows)=10 == min(highs)=10 → 真相切。
    // #246 新口径（含端点 `<=`）判**有重合**；口径回改旧 `<` 时本条立刻红。
    let a = stroke_hl(Direction::Up, 0, 4, 10.0, 5.0);
    let b = stroke_hl(Direction::Down, 4, 8, 12.0, 8.0);
    let c = stroke_hl(Direction::Up, 8, 12, 15.0, 10.0);
    let lo = a.low.max(b.low).max(c.low);
    let hi = a.high.min(b.high).min(c.high);
    assert_eq!(
        lo, hi,
        "前件：本用例须是真相切（max(lows)==min(highs)），否则断言退化"
    );
    assert!(
        three_stroke_overlap(&a, &b, &c),
        "三笔互异真相切（max_lo==min_hi==10）须判有重合——#246 新口径 `<=`；若红，\
         `three_stroke_overlap` 被改回旧口径严格 `<`"
    );
}

#[test]
fn three_stroke_overlap_pairwise_tangent() {
    // 相切由**两笔**取得、第三笔宽包含（[5,10] / [10,20] / [0,30]）：max_lo=10 == min_hi=10。
    // 与上一条互补：相切点的来源不同（两笔 vs 三笔共同取得），同样锁含等号。
    let a = stroke_hl(Direction::Up, 0, 4, 10.0, 5.0);
    let b = stroke_hl(Direction::Down, 4, 8, 20.0, 10.0);
    let c = stroke_hl(Direction::Up, 8, 12, 30.0, 0.0);
    let lo = a.low.max(b.low).max(c.low);
    let hi = a.high.min(b.high).min(c.high);
    assert_eq!(lo, hi, "前件：本用例须是真相切，否则断言退化");
    assert!(
        three_stroke_overlap(&a, &b, &c),
        "两笔相切于 10（第三笔宽包含）须判有重合——#246 新口径 `<=`；若红，口径被改回旧 `<`"
    );
}

#[test]
fn three_stroke_overlap_strict_cases_unchanged() {
    // 对照组：严格重叠（max_lo < min_hi）仍 true、严格分离（max_lo > min_hi）仍 false。
    // 新旧口径一致 ⟹ 本条对口径篡改**不敏感**，只作回归护栏，不计入相切锁。
    let a = stroke_hl(Direction::Up, 0, 4, 15.0, 5.0);
    let b = stroke_hl(Direction::Down, 4, 8, 12.0, 8.0);
    let c = stroke_hl(Direction::Up, 8, 12, 18.0, 7.0);
    assert!(three_stroke_overlap(&a, &b, &c), "严格重叠须判有重合");

    let d = stroke_hl(Direction::Up, 0, 4, 10.0, 5.0);
    let e = stroke_hl(Direction::Down, 4, 8, 20.0, 12.0);
    let f = stroke_hl(Direction::Up, 8, 12, 30.0, 22.0);
    assert!(
        !three_stroke_overlap(&d, &e, &f),
        "严格分离（max_lo=22 > min_hi=10）须判无重合"
    );
}

#[test]
fn is_fractal_and_gap_up_tangent_no_gap() {
    // 向上段顶分型，a 与 b 真相切（b_l == a_h == 10）。
    // #246 新口径（缺口臂严格 `>`）判**无缺口**；口径回改旧 `>=` 时本条立刻红。
    // 先 assert 分型前件成立——前件不成立时 has_gap 恒 false，断言会退化为同义反复。
    let (a_h, a_l) = (10.0, 5.0);
    let (b_h, b_l) = (20.0, 10.0);
    let (c_h, c_l) = (12.0, 6.0);
    assert_eq!(b_l, a_h, "前件：本用例须是真相切（b_l == a_h）");
    let (is_fractal, has_gap) = is_fractal_and_gap(a_h, a_l, b_h, b_l, c_h, c_l, Direction::Up);
    assert!(
        is_fractal,
        "前件：向上段顶分型须成立，否则 has_gap 断言退化"
    );
    assert!(
        !has_gap,
        "向上段 a-b 真相切（b_l == a_h == 10）须判无缺口——#246 新口径严格 `>`；若红，\
         `is_fractal_and_gap` 向上臂被改回旧口径 `>=`"
    );
}

#[test]
fn is_fractal_and_gap_down_tangent_no_gap() {
    // 向下段底分型，a 与 b 真相切（a_l == b_h == 10）。同上，锁向下臂严格 `>`。
    let (a_h, a_l) = (20.0, 10.0);
    let (b_h, b_l) = (10.0, 2.0);
    let (c_h, c_l) = (18.0, 8.0);
    assert_eq!(a_l, b_h, "前件：本用例须是真相切（a_l == b_h）");
    let (is_fractal, has_gap) = is_fractal_and_gap(a_h, a_l, b_h, b_l, c_h, c_l, Direction::Down);
    assert!(
        is_fractal,
        "前件：向下段底分型须成立，否则 has_gap 断言退化"
    );
    assert!(
        !has_gap,
        "向下段 a-b 真相切（a_l == b_h == 10）须判无缺口——#246 新口径严格 `>`；若红，\
         `is_fractal_and_gap` 向下臂被改回旧口径 `>=`"
    );
}

#[test]
fn is_fractal_and_gap_strict_cases_unchanged() {
    // 对照组：严格跳空两方向仍判有缺口（新旧口径一致），对口径篡改不敏感。
    let (f_up, g_up) = is_fractal_and_gap(10.0, 5.0, 20.0, 12.0, 12.0, 6.0, Direction::Up);
    assert!(f_up, "前件：向上段顶分型须成立");
    assert!(g_up, "向上段严格跳空（b_l=12 > a_h=10）须判有缺口");

    let (f_dn, g_dn) = is_fractal_and_gap(20.0, 12.0, 10.0, 2.0, 18.0, 8.0, Direction::Down);
    assert!(f_dn, "前件：向下段底分型须成立");
    assert!(g_dn, "向下段严格跳空（a_l=12 > b_h=10）须判有缺口");
}
