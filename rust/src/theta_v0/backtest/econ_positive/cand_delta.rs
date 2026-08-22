//! Cand^δ_ℓ 判据族（#1175 A01 职责块自 `econ_positive.rs` 迁出，零行为）。
//!
//! 承载 [`BspCandType`] 分派键与 per-rung/base 候选谓词。消费面经 `econ_positive`
//! 重导出保持原路径。

use super::*;

/// 673-fix（codex 裁决①§673-fix）：Cand^δ_ℓ 候选类型——接口级三分拆的分派键。
///
/// bits 非互斥（P4§5 六买卖点可重合），故按优先级 Type1>Type2>Type3 坍缩到单一候选类型。
/// 保持旧 `is_type1` 语义：buy1/sell1 置位即走 Type1 的 `div_cand` 路径（bit-exact 不动）。
/// StructBreak（codex 终局裁决A，2026-07-02）：`class_index()==0` 的零 bit 破中枢未背驰候选
/// （P2-R2，`signal.rs:577-580`）——概念上与 Type3（未破核心区间回试）几何前提互斥，
/// 不可再落 `else => Type3`（673 号先例：互斥语义混入同一分支须拆分谓词/分支）。
/// 门控：样本层/`MuClass.bsp_class()==0` 统计保留，τ 门控层恒拒（无 BSP 证书）。
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in super::super) enum BspCandType {
    Type1,
    Type2,
    Type3,
    StructBreak,
}

/// 从 `BspBits`+方向派生候选类型（优先级 StructBreak（六 bit 全零）>Type1>Type2>Type3）。
pub(super) fn bsp_cand_type(bits: &BspBits, delta: Side) -> BspCandType {
    if bits.class_index() == 0 {
        return BspCandType::StructBreak;
    }
    match delta {
        Side::Long => {
            if bits.buy1 {
                BspCandType::Type1
            } else if bits.buy2 {
                BspCandType::Type2
            } else {
                BspCandType::Type3
            }
        }
        Side::Short => {
            if bits.sell1 {
                BspCandType::Type1
            } else if bits.sell2 {
                BspCandType::Type2
            } else {
                BspCandType::Type3
            }
        }
    }
}

/// 673-fix Type1 候选谓词：本级趋势背驰段（区间套原文对象，606 号有效域）。
///
/// per-rung `Cand^δ_k`——在 rung 的次级别走势序列 `rung_subs` 中找 `end_index==source_index`
/// 的执行级候选段，跑 `div_cand`（Extreme+Weak 四条件）。bit-exact 复用旧 Type1 分支逻辑。
fn cand_delta_type1_extreme(
    rung_subs: &[crate::theta_v0::classifier::recursive_tower::LeveledMove],
    source_index: usize,
    delta: Side,
    hist: &[f64],
    strokes: &[crate::theta_v0::types::Stroke],
    parent_center: Option<&crate::theta_v0::types::Center>,
    gauge: crate::theta_v0::classifier::divergence::DivergenceGauge,
) -> bool {
    match find_move_containing_index(rung_subs, source_index) {
        Some(tidx) => crate::theta_v0::classifier::cand_predicate::div_cand(
            &crate::theta_v0::classifier::cand_predicate::DivCandInput {
                context: rung_subs,
                target_idx: tidx,
                hist,
                delta,
                strokes,
                parent_center, // ★#883：D-3 取段的「界」= rung 父走势（knode）的最近中枢
                gauge,
            },
        ),
        None => false, // 执行级候选段不在 k 级次级别序列中 ⟹ 无 Cand
    }
}

/// 673-fix Type2 候选谓词：一类点后回抽走势完成（第17课L60 完备性）。
///
/// **保护边界 = 一类点极值**（回抽不破一类点）——由上游结构分类器置 buy2/sell2 位时强制，
/// 本谓词不在 Cand 层重门保护位（no-patch 双门）。存在性锚 = 次级别 Type1（定律一下沉，
/// 第29课L396「二三类精确点要下次级别以下找第一类」）。`lvl==0`（塔内无次级别——塔不从笔递归：
/// 笔由解析层产出（`ParseLayer.strokes`）但不在塔的级别阶梯内，系构造选择，非客观无次级别（订正 #520，
/// 原注「无次级别、递归底」经 #450 查实为假））⟹ 存在性
/// 免门（Type2@level0 不误拒）；`NoDivergence`（无锚）= 小转大（该级无一类精确点无法下沉定位）⟹ 门拒。
///
/// ★S4 接线（#802 空洞①/②）：下钻返回 [`DescendLocator`]——门消费存在性（`.anchored()`），
/// 深度与终止成因随 [`DescendStop`] 带出；`lvl==0` 显式写成 **L0 天花板（基底选择，#520）**，
/// 不再与「递归底」混名。不变式（#846 B 类零反例）在消费层 `debug_assert_ne!` 报错。
pub(super) fn cand_delta_type2_completion(
    s: &crate::theta_v0::classifier::recursive_tower::LeveledMove,
    source_index: usize,
    delta: Side,
    hist: &[f64],
    lvl: usize,
    strokes: &[crate::theta_v0::types::Stroke],
    gauge: crate::theta_v0::classifier::divergence::DivergenceGauge,
) -> bool {
    if lvl == 0 {
        // L0 天花板（基底选择——塔以线段为底、笔不在塔级别阶梯内，#520 订正）⟹ 存在性免门。
        return true;
    }
    let locator = descend_type1_anchor_depth(s, source_index, delta, hist, strokes, gauge);
    debug_assert_ne!(
        locator.stop(),
        DescendStop::NoAlign,
        "Type2 下钻：L0 以上取不到包含段（#846 B 类零反例 36175/36175）"
    );
    locator.anchored()
}

/// 673-fix Type3 候选谓词：离开中枢后回抽/反抽走势完成（**独立分支**，codex 裁决①）。
///
/// **保护边界 = 中枢 ZG/ZD**（离开中枢回抽不入 `c.zg`/`c.zd`，几何见 `descend.rs`
/// `sub_broke_above`/`sub_broke_below`）——与 Type2 的「一类点极值」锚点/失效条件不同，故 Type3
/// **不复用 Type2 顶层谓词**（codex：「Type2 保护位是一类点极值，Type3 保护边界是中枢区间边界」）。
/// ZG/ZD 边界由上游 `bsp.rs`（`endpoint_to_bsp`）置 buy3/sell3 位时强制（V型反转回试不入中枢已判），本谓词不在
/// Cand 层重门 ZG/ZD（上游已滤 ⟹ 双门=dead gate，信号集差 0）——保护边界**归属**记录于此，语义与
/// Type2 分离。存在性锚复用 `descend_type1_anchor_depth`（codex 允许「Type3 最多复用 Type2 的
/// 反向走势完成 helper」）：精确点=次级别 Type1（定律一下沉）；`lvl==0` 免门，`NoDivergence`=小转大门拒。
///
/// ★S4 接线（#802 空洞①/②）：同 [`cand_delta_type2_completion`]——`lvl==0` = L0 天花板（#520
/// 基底选择），下钻门消费存在性（`.anchored()`），不变式（#846 B 类零反例）在消费层报错。
pub(super) fn cand_delta_type3_retest(
    s: &crate::theta_v0::classifier::recursive_tower::LeveledMove,
    source_index: usize,
    delta: Side,
    hist: &[f64],
    lvl: usize,
    strokes: &[crate::theta_v0::types::Stroke],
    gauge: crate::theta_v0::classifier::divergence::DivergenceGauge,
) -> bool {
    if lvl == 0 {
        // L0 天花板（基底选择——塔以线段为底、笔不在塔级别阶梯内，#520 订正）⟹ 存在性免门。
        return true;
    }
    let locator = descend_type1_anchor_depth(s, source_index, delta, hist, strokes, gauge);
    debug_assert_ne!(
        locator.stop(),
        DescendStop::NoAlign,
        "Type3 下钻：L0 以上取不到包含段（#846 B 类零反例 36175/36175）"
    );
    locator.anchored()
}

/// 673-fix 薄 dispatcher：per-rung `Cand^δ_k` 按候选类型分派（不承载判据逻辑，codex 裁决①）。
///
/// Type1 → 本级背驰段 `div_cand`；Type2/3 存在性已由 [`cand_delta_base_gate`] 门控（base 一次），
/// 上级 rung 载上级语境（第17课L60 完备性保证 Type2/3 存在）⟹ cand=true。
#[allow(clippy::too_many_arguments)]
pub(super) fn cand_delta(
    cand_type: BspCandType,
    rung_subs: &[crate::theta_v0::classifier::recursive_tower::LeveledMove],
    source_index: usize,
    delta: Side,
    hist: &[f64],
    strokes: &[crate::theta_v0::types::Stroke],
    parent_center: Option<&crate::theta_v0::types::Center>,
    gauge: crate::theta_v0::classifier::divergence::DivergenceGauge,
) -> bool {
    match cand_type {
        BspCandType::Type1 => cand_delta_type1_extreme(
            rung_subs,
            source_index,
            delta,
            hist,
            strokes,
            parent_center,
            gauge,
        ),
        BspCandType::Type2 | BspCandType::Type3 => true,
        BspCandType::StructBreak => false, // 门拒（codex 终局裁决A）：无对应确认语义，不复用 Type3 锚
    }
}

/// 673-fix base 存在性门（一次，非 per-rung）：Type2/3 定律一下沉锚定，按类型分派。
///
/// ★S4 接线（#802 空洞③「Type1 从不下钻」）：Type1 也走一次向下定位器——但下钻是**定位工具**
/// 不是门（ADR 0013 裁定七：区间套是主动调用的定位工具；探针 #852/#870 实测 Type1 首步
/// DivFalse ≈92.7% ⟹ 把下钻结果当准入门会误杀几乎全部 Type1 信号）。故 Type1 恒 true（背驰判定
/// 在 per-rung `div_cand`），下钻只执行不变式守卫（NoAlign 报错）+ 供诊断消费深度（终止读数）。
/// Type2/3 委托各自谓词（存在性锚 + 保护边界归属记录）。返回 false ⟹ 整证书拒（小转大：该级
/// 无一类精确点无法下沉定位）。
#[allow(clippy::too_many_arguments)]
pub(super) fn cand_delta_base_gate(
    cand_type: BspCandType,
    s: &crate::theta_v0::classifier::recursive_tower::LeveledMove,
    source_index: usize,
    delta: Side,
    hist: &[f64],
    lvl: usize,
    strokes: &[crate::theta_v0::types::Stroke],
    gauge: crate::theta_v0::classifier::divergence::DivergenceGauge,
) -> bool {
    match cand_type {
        BspCandType::Type1 => {
            // 下钻执行一次（「Type1 要走」）；NoAlign 不变式在此消费层报错，结果不作准入门。
            let locator = descend_type1_anchor_depth(s, source_index, delta, hist, strokes, gauge);
            debug_assert_ne!(
                locator.stop(),
                DescendStop::NoAlign,
                "Type1 下钻：L0 以上取不到包含段（#846 B 类零反例 36175/36175）"
            );
            true
        }
        BspCandType::Type2 => {
            cand_delta_type2_completion(s, source_index, delta, hist, lvl, strokes, gauge)
        }
        BspCandType::Type3 => {
            cand_delta_type3_retest(s, source_index, delta, hist, lvl, strokes, gauge)
        }
        BspCandType::StructBreak => false, // 门拒（codex 终局裁决A）：无对应确认语义，不复用 Type3 锚
    }
}
