//! ★S4 向下定位器（#1175 A01 职责块自 `econ_positive.rs` 迁出，零行为）。
//!
//! 承载 [`DescendLocator`]/[`DescendStop`] 与定律一下沉锚定
//! [`descend_type1_anchor_depth`]。消费面经 `econ_positive` 重导出保持原路径。

use super::*;

/// ★S4 向下定位器（区间套下钻）的落地结果——「一重 ＝ ⟨挂载层 k，横向读法，向下定位器，一份筹码⟩」
/// （ADR 0011）第三格的真消费形态（#802 空洞①「深度全弃」的接线）。
///
/// **S5 命名分家**：本类型是**向下**（`sub_moves` 逐级下沉定位）；与**向上**的 `NestCertificate.rungs`
/// （`MuClass.nest_depth` = `rungs.len()`）**同名不同向**——两者字面都叫「深度」，历史上被已归档
/// 报告 `l2-depth-distribution-20260702.md` §5 写成「同一件事的两次测量」（ADR 0013 裁定七钉死混淆）。
/// 本类型以独立类型 + 明确方向名挡混用：向上 = rungs 数（`u8`，`MuClass.nest_depth`），向下 = 本类型。
/// `pub(super)`：供 `backtest` 树内跨模块引用（如 `mu_estimator::MuClass.nest_depth` 的 S5 分名注释）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in super::super) struct DescendLocator {
    /// 连续锚定层数 d（≥1 = 至少锚定到次级别 Type1/类一类点；0 = 首级即终止）。
    depth: usize,
    /// 递归停止成因（终止条件 = 成本门 + L0 天花板，见 [`DescendStop`]）。
    stop: DescendStop,
}

impl DescendLocator {
    /// 是否至少锚定一级（旧 `Option::is_some` 语义：次级别 Type1 锚存在）。
    pub(super) fn anchored(&self) -> bool {
        self.depth >= 1
    }

    /// 下沉深度（0 = 首级即终止）。
    pub(super) fn depth(&self) -> usize {
        self.depth
    }

    /// 递归停止成因。
    pub(super) fn stop(&self) -> DescendStop {
        self.stop
    }
}

/// 下钻终止成因（复用 #846 失败分类 A/B/C 三分，同探针 `StepFail` 口径，**不另造**）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in super::super) enum DescendStop {
    /// A：`sub_moves` 空 = 到达 L0 天花板（塔以线段为底、笔不在塔级别阶梯内，#520 基底选择）。
    /// **正常终止，不算断链**。
    BaseL0,
    /// B：`sub_moves` 非空但**既无 `end_index == source_index` 对齐段、也无包含段**（source_index
    /// 落在 sub_moves 覆盖区间之外）——塔构造不变量（父段末尾恒等于子段末尾）违反。#846 坐实
    /// B 类零反例 36175/36175 ⟹ 「L0 以上取不到段即报错」不变式的报错对象（报错守卫在生产消费层
    /// `debug_assert_ne!` 落地，本函数保持纯函数返回 `NoAlign`，诊断层可继续计数 B 类）。
    /// ★#1076（#1028 终局）：点锚迁移后 departure 终点可能落次级别走势**内部**（非段边界），
    /// 该情形由「区间包含」回退消化（见 [`descend_type1_anchor_depth`]），**不再落 NoAlign**——
    /// 落 NoAlign 仅剩真正的覆盖缺口（塔结构不变量违反）。
    NoAlign,
    /// C：对齐/包含段存在但 `div_cand` 判假——判据层合法终止（成本门/市场现实，非结构错误）。
    NoDivergence,
}

/// 673 号段2：定律一下沉锚定深度（第29课L396「二三类精确的都要下次级别以下找第一类」）。
///
/// Type2/3@ℓ 的精确点 = 次级别 Type1（回抽这个次级别走势的结束点=次级别一类背驰点，定律一 第17课L66）。
/// 从候选段 `s`（=回抽次级别走势，`end_index==source_index`）向次级别下钻：在 `s.sub_moves`
/// （次级别 ℓ-1 走势序列）中找 `end_index==source_index` 的段（★#1076：MISS 回退「区间包含」定位），跑**完整** `div_cand`（Extreme+Weak
/// 四条件，含盘整背驰——背驰段定义第27课L21 涵盖趋势/盘整，非弱化版）。真递归下沉：锚定成立后继续
/// 钻入该次级别 Type1 段，逐级收缩到最低可用级别（`sub_moves` 空=递归底 level0）。
///
/// ★S4 接线（SPEC #847）：返回值从 `Option<usize>` 升为 [`DescendLocator`]——**深度与终止成因
/// 一起带出**（不再 `.is_some()` 只问能不能钻，#802 空洞①）。终止条件 = 成本门（经济）+
/// L0 天花板（基底选择，#817 N-2 裁定四 阶段一：现役塔 L0–L4 上成本门不咬任何级，#907 实测，
/// 故终止实际先到 L0 天花板；成本门参数随执行系统而定，**不钉死级别**——钉死 = 把参数写死成常数）。
///
/// - `DescendLocator{depth: d, ..}`（d≥1）：次级别 Type1 锚点成立，区间套逐级收缩穿越 d 层
///   （d=最低可用级别的下沉深度）；`stop` 记递归在 d 层之后为何停止。
/// - `stop` 三成因（复用 #846 失败分类 A/B/C，不另造）：
///   - [`DescendStop::BaseL0`]（A）：`sub_moves` 空 = 到达 L0 天花板（#520 基底选择，正常终止）。
///   - [`DescendStop::NoDivergence`]（C）：对齐/包含段存在但 `div_cand` 判假——判据层合法终止
///     （成本门/市场现实）。该级无一类买卖点、精确点无法下沉定位时走小转大通道
///     （[`build_xzd_fallback`]，知识库 L410「区间套和背驰不可解释情况的补充」）。
///   - [`DescendStop::NoAlign`]（B）：L0 以上既取不到 `end_index==source_index` 对齐段、也取不到
///     包含段（source_index 落在 sub_moves 覆盖区间之外）——塔构造不变量违反，不变式报错对象
///     （见 [`DescendStop`]）。★#1076：departure 终点落次级别走势**内部**（非边界）的 54 例
///     由「区间包含」回退消化，不落 NoAlign。
///
/// 区间套 `[J_{ℓ-1}⊆J_ℓ]` 由下钻**结构性保证**：`sub_move` 的 `[start,end]` ⊆ parent 的 `[start,end]`
/// （recursive_tower Compose 由连续 `sub_moves` 组装的不变量），故不重复 `is_sub` 检查（invariant 非条件）。
///
/// **认识论 L0**：纯结构下钻 + 确定性 `div_cand`。复用 N^δ 上钻路径同一 `div_cand` 判据
/// （no-patch，非平行简化版；上钻找 parent，下钻用 sub_moves，是同一区间套的对偶方向）。
pub(super) fn descend_type1_anchor_depth(
    s: &crate::theta_v0::classifier::recursive_tower::LeveledMove,
    source_index: usize,
    delta: Side,
    hist: &[f64],
    strokes: &[crate::theta_v0::types::Stroke],
    gauge: crate::theta_v0::classifier::divergence::DivergenceGauge,
) -> DescendLocator {
    let subs = s.sub_moves.as_slice();
    if subs.is_empty() {
        // L0 天花板（基底选择——塔以线段为底、笔不在塔级别阶梯内，#520 订正），正常终止。
        return DescendLocator {
            depth: 0,
            stop: DescendStop::BaseL0,
        };
    }
    // 次级别 Type1 背驰段判据：s.sub_moves 中 end_index==source_index 段跑完整 div_cand。
    // ★#883：D-3 取段的「界」= 父走势 s 的最近中枢（每级递归各取各的父中枢）。
    // ★#1076（#1028 终局）：点锚迁移后 departure 单元终点可能落在次级别走势**内部**（不落任何
    // sub_moves 段边界，三轮实测 396 信号 54 例）⟹ `end ==` 精确匹配 NoAlign 误断、下钻对齐失败
    // 被排除。回退「区间包含」定位（find_move_containing_index）——只动锚定位、不动 div_cand 判据链。
    let tidx = match find_move_by_end_index(subs, source_index) {
        Some(tidx) => tidx,
        None => match find_move_containing_index(subs, source_index) {
            Some(tidx) => tidx,
            None => {
                // B 类：既无 end== 对齐段、也无包含段（source_index 落在 sub_moves 覆盖区间之外）
                // = 结构不变量违反（不变式报错对象，守卫在生产消费层）。
                return DescendLocator {
                    depth: 0,
                    stop: DescendStop::NoAlign,
                };
            }
        },
    };
    let anchor_ok = crate::theta_v0::classifier::cand_predicate::div_cand(
        &crate::theta_v0::classifier::cand_predicate::DivCandInput {
            context: subs,
            target_idx: tidx,
            hist,
            delta,
            strokes,
            parent_center: crate::theta_v0::classifier::cand_predicate::parent_last_center(s),
            gauge,
        },
    );
    if !anchor_ok {
        // C 类：次级别无一类背驰锚点 ⟹ 判据层合法终止（成本门/市场现实）。
        return DescendLocator {
            depth: 0,
            stop: DescendStop::NoDivergence,
        };
    }
    // 真递归下沉：钻入该次级别 Type1 段，逐级收缩到最低可用级别（深层无锚/到底 ⟹ 本级即最低可用锚）。
    let deeper = descend_type1_anchor_depth(&subs[tidx], source_index, delta, hist, strokes, gauge);
    DescendLocator {
        depth: deeper.depth + 1,
        stop: deeper.stop,
    }
}
