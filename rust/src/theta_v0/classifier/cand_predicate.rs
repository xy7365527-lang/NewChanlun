//! DivCand^δ_{Θ,ℓ}(s,t)：背驰段候选谓词（Cand 判据接入，W1 工位）。
//!
//! ## 定义（三方交叉确认：Lead 推导 + codex C1 裁决 + ChatGPT 推导一致）
//!
//! `DivCand^δ_{Θ,ℓ}(s,t)` :=
//!   [dir(s) = −δ]                             ← 条件1：方向反（δ=交易方向，背驰段走势方向）
//!   ∧ [∃s'∈S^vis: Comparable_ℓ(s',s,t)]      ← 条件2：存在同上级语境可比较前段 s'
//!   ∧ [Extreme^δ(s',s,t)]                     ← 条件3：价格极值更进一步
//!   ∧ [Weak^δ_{Θ,ℓ}(s,s',t)]                 ← 条件4：MACD 面积力度衰减（Weak = Diverge）
//!
//! ## 操作化
//!
//! 给定上级走势（Compose）的次级别走势序列 `context`（来自 `LeveledMove.sub_moves`）和目标
//! 段在序列中的索引 `target_idx`：
//!
//! - **条件1**：`rmove_dir(context[target_idx])` 与 δ 方向相反
//!   - δ=Long(买)  →   dir(s) = Down  （下跌段末端背驰买）
//!   - δ=Short(卖) →   dir(s) = Up    （上涨段末端背驰卖）
//! - **条件2**：在 `context[..target_idx]` 中找最近的同向段 s'（dir(s') == dir(s)）
//!   ← Comparable = 同一 Compose 父下的前一同向次级别走势
//! - **条件3**：
//!   - δ=Long：  `lo(s) < lo(s')`  （s 低点更低——下跌更深）
//!   - δ=Short：`hi(s) > hi(s')`  （s 高点更高——上涨更高）
//! - **条件4**：MACD hist 面积：`area(s) < area(s')`（力度衰减）
//!   ← 复用 [`super::divergence::segment_macd_area`] + [`super::divergence::is_divergence`]
//!
//! ## 认识论等级（formalization-validity-domain 231号）
//!
//! - **L0**：定义操作化（条件 1/2/3 是结构谓词，逻辑必然）。
//! - **L0**（条件4 MACD 口径）：MACD 面积作力度代理是 Θ_MACD 参数化选择（非唯一真实力度），
//!   但判定本身是确定性算术。alpha 有效性待 W-VERIFY L2/L3，此处不声明 alpha。
//! - **有效域边界**：有效域 ⊆ 定义域。定义域=全部 LeveledMove 序列；有效域=`context.len()>=2`
//!   且前序中存在同向段（条件2 可满足）——否则 Cand=false（非 bug，合法定位失败）。
//!
//! ## 结果包（六要素）
//!
//! - **结论**：返回 `bool`（DivCand 四条件合取真值）。
//! - **定义依据**：三方交叉确认规格（codex C1 裁决 2026-07-01，Lead 推导，ChatGPT 推导一致）；
//!   条件1 参照方向定义（δ=交易方向，背驰段方向 = −δ）；条件4 参照 divergence.rs MACD 面积。
//! - **边界条件**：(1) `context.len() < 2` ⟹ false（无前段可比较）；
//!   (2) 无同向前段 ⟹ false（条件2 不满足）；
//!   (3) MACD hist 为空 ⟹ area=0.0 ⟹ 0 < 0 = false（条件4 不满足）；
//!   (4) 方向翻转（δ Long↔Short）⟹ 条件1 方向判定翻转；
//!   (5) 上游更换 Θ（如 Θ_Force 次级别力度）时，条件4 接口保持，值可变。
//! - **下游推论**：Cand=true ⟹ NestRung.cand=true ⟹ NestCertificate.n_delta() 可为 true；
//!   Cand=false ⟹ n_delta()=false（spec N^δ 定义，任一级 Cand=0 ⟹ 整体 0）。
//! - **谱系引用**：W1 工位规格（三方一致，2026-07-01）；codex C1 裁决。不确定是否有相关
//!   概念分离谱系，保守声明。
//! - **影响声明**：新建本文件；暴露 `div_cand`/`bsp_div_cand`/`ContextMove`/`rmove_dir` 供
//!   `econ_positive.rs::build_multilevel_nest_cert`（W1 返工实际入口）调用；
//!   不改 divergence.rs / nest.rs / descend.rs；不碰识别层。L0 结构谓词。

use std::rc::Rc;

use super::descend::RMove;
use super::divergence::{is_divergence, segment_macd_area};
use super::recursive_tower::LeveledMove;
use super::super::types::{Direction, Side};

/// δ 交易方向（Long=买/+1，Short=卖/−1）。
/// 复用 types::Side（与 NestCertificate.side 同类型）。
pub use super::super::types::Side as Delta;

/// DivCand^δ 单次判定所需上下文（从塔提取）。
///
/// `context`：候选段所在上级走势的次级别走势序列（`parent.sub_moves`）。
/// `target_idx`：候选段在 `context` 中的索引（`context[target_idx]` 是目标段 s）。
/// `hist`：MACD hist 序列（全 bar 域，bar 索引对齐）。
/// `delta`：交易方向 δ（Long=买候选找下跌背驰，Short=卖候选找上涨背驰）。
pub struct DivCandInput<'a> {
    /// 候选段所在上级走势的次级别走势序列（来自 `LeveledMove.sub_moves`）。
    pub context: &'a [ContextMove],
    /// 目标段在 `context` 中的索引。
    pub target_idx: usize,
    /// MACD hist 序列（全 bar 域，从 bar 0 开始）。
    pub hist: &'a [f64],
    /// 交易方向 δ。
    pub delta: Delta,
}

/// 简化的走势坐标（从 `LeveledMove` 提取，只携带 DivCand 需要的字段）。
///
/// 不持有 `LeveledMove` 引用（避免递归借用），调用方显式提取。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextMove {
    /// 走势方向（Segment.direction / Compose 外缘派生，见 `rmove_dir`）。
    pub direction: Direction,
    /// 走势区间下沿（= `RMove::lo()`）。
    pub lo: i64,
    /// 走势区间上沿（= `RMove::hi()`）。
    pub hi: i64,
    /// 该走势覆盖的 bar 起点（L0 原始 K 序，来自 `LeveledMove.start_index`）。
    pub start_index: usize,
    /// 该走势覆盖的 bar 终点（L0 原始 K 序，来自 `LeveledMove.end_index`）。
    pub end_index: usize,
}

impl ContextMove {
    /// 从 `RMove` + 坐标构建（调用方从 `LeveledMove` 提取）。
    pub fn from_rmove(rmove: &RMove, start_index: usize, end_index: usize) -> Self {
        ContextMove {
            direction: rmove_dir(rmove),
            lo: rmove.lo(),
            hi: rmove.hi(),
            start_index,
            end_index,
        }
    }
}

/// 走势方向判定（Segment 直接取；Compose 取外缘趋势——首次级 hi vs 末次级 hi）。
///
/// Compose 无 `direction` 字段（Lean μF 设计，方向由外缘判据派生），与 `fold_direction`
/// / `rmove_side` 同口径。Compose.subs 空 ⟹ 缺省 Up（结构占位，与 extract_elements 同）。
pub fn rmove_dir(rmove: &RMove) -> Direction {
    match rmove {
        RMove::Segment { direction, .. } => *direction,
        RMove::Compose { subs, .. } => {
            match (subs.first(), subs.last()) {
                (Some(f), Some(l)) if l.hi() >= f.hi() => Direction::Up,
                (Some(_), Some(_)) => Direction::Down,
                _ => Direction::Up, // 空 subs 占位
            }
        }
    }
}

/// DivCand^δ_{Θ,ℓ}(s,t)：背驰段候选四条件合取谓词。
///
/// ## 四条件
/// 1. **方向**：`dir(s) = −δ`（δ=Long→s 方向 Down；δ=Short→s 方向 Up）
/// 2. **Comparable**：`context[..target_idx]` 中存在最近同向段 s'
/// 3. **Extreme**：δ=Long→`lo(s)<lo(s')`；δ=Short→`hi(s)>hi(s')`
/// 4. **Weak**（Θ_MACD）：`area(hist, s) < area(hist, s')`
///
/// 任一条件不满足 ⟹ false（`Cand=0`，合法定位失败，非 bug）。
pub fn div_cand(input: &DivCandInput<'_>) -> bool {
    let DivCandInput { context, target_idx, hist, delta } = input;
    let context = *context;
    let target_idx = *target_idx;
    let hist = *hist;
    let delta = *delta;

    // 越界保护。
    if target_idx >= context.len() {
        return false;
    }
    let s = &context[target_idx];

    // 条件1：dir(s) = −δ。
    let expected_dir = match delta {
        Side::Long => Direction::Down,  // δ=买 → 背驰段方向 = 下跌
        Side::Short => Direction::Up,   // δ=卖 → 背驰段方向 = 上涨
    };
    if s.direction != expected_dir {
        return false;
    }

    // 条件2：找前序最近同向段 s'（Comparable = 同父次级别序列中前序最近同向段）。
    // ponytail: 取最近（最大 i < target_idx，s.direction == s'.direction）。
    let prev = context[..target_idx]
        .iter()
        .rfind(|m| m.direction == s.direction);
    let Some(s_prev) = prev else { return false };

    // 条件3：Extreme。
    let extreme_ok = match delta {
        Side::Long => s.lo < s_prev.lo,   // 下跌段更低低点
        Side::Short => s.hi > s_prev.hi,  // 上涨段更高高点
    };
    if !extreme_ok {
        return false;
    }

    // 条件4：Weak（Θ_MACD，力度衰减）。
    let prev_area = segment_macd_area(hist, s_prev.start_index, s_prev.end_index);
    let curr_area = segment_macd_area(hist, s.start_index, s.end_index);
    is_divergence(prev_area, curr_area)
}

/// 从塔（`tower`）定位 BspPoint 对应的候选段，计算 DivCand^δ。
///
/// ## 查找逻辑
///
/// 1. 在 `tower[level]` 中找 `end_index == source_index` 的 LeveledMove（候选段 s）。
/// 2. 在 `tower[level+1]`（上级）中找包含 s 的 Compose，取其 `sub_moves` 作为 context。
///    ★若无上级层或无包含 s 的 Compose ⟹ false（无上级语境无法比较前段，合法定位失败）。
/// 3. 在 context 中找 `end_index == source_index` 的位置（= `target_idx`）。
/// 4. 调用 `div_cand`。
///
/// ## 边界条件
///
/// - `tower.len() <= level`（level 不存在）⟹ false
/// - `tower[level]` 中无 `end_index == source_index` 的走势 ⟹ false
/// - `tower[level+1]` 不存在或其中无包含 s 的 Compose ⟹ false（无上级语境）
/// - DivCand 任一条件不满足 ⟹ false
///
/// ## 认识论等级（L0）
///
/// 纯结构查找 + DivCand 确定性算术。alpha 有效性待 L2/L3，不声明 alpha。
pub fn bsp_div_cand(
    tower: &[Rc<Vec<LeveledMove>>],
    level: usize,
    source_index: usize,
    delta: Delta,
    hist: &[f64],
) -> bool {
    // 1. 找候选段 s（end_index == source_index）。
    let level_moves = tower.get(level).map(|rc| rc.as_slice()).unwrap_or(&[]);
    let Some(target_pos) = level_moves.iter().position(|m| m.end_index == source_index) else {
        return false;
    };
    let s = &level_moves[target_pos];

    // 2. 在上级（tower[level+1]）找包含 s 的 Compose，取其 sub_moves 作 context。
    let upper_moves = tower.get(level + 1).map(|rc| rc.as_slice()).unwrap_or(&[]);
    let Some(parent) = upper_moves.iter().find(|p| {
        p.start_index <= s.start_index && p.end_index >= s.end_index
    }) else {
        return false; // 无上级语境
    };
    let context_subs = &parent.sub_moves;
    if context_subs.is_empty() {
        return false;
    }

    // 3. 将 sub_moves 转换为 ContextMove 序列，找 target_idx（end_index 匹配）。
    let ctx: Vec<ContextMove> = context_subs.iter().map(|m| {
        ContextMove::from_rmove(&m.rmove, m.start_index, m.end_index)
    }).collect();
    // ponytail: end_index 唯一性在同一 sub_moves 序列内假设成立（Compose 窗口不重叠）。
    let Some(target_idx) = context_subs.iter().position(|m| m.end_index == source_index) else {
        return false;
    };

    div_cand(&DivCandInput { context: &ctx, target_idx, hist, delta })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 测试工具 ──────────────────────────────────────────────────────────────

    fn down_seg(lo: i64, hi: i64, start: usize, end: usize) -> ContextMove {
        ContextMove { direction: Direction::Down, lo, hi, start_index: start, end_index: end }
    }

    fn up_seg(lo: i64, hi: i64, start: usize, end: usize) -> ContextMove {
        ContextMove { direction: Direction::Up, lo, hi, start_index: start, end_index: end }
    }

    /// hist 序列：每 bar 固定值，面积 = 值 × bar 数。
    fn flat_hist(val: f64, n: usize) -> Vec<f64> {
        vec![val; n]
    }

    // ── 条件1：方向反（dir(s) = −δ） ──────────────────────────────────────────

    /// δ=Long 时，候选段方向必须为 Down；若方向为 Up ⟹ Cand=0。
    #[test]
    fn cond1_wrong_dir_long_delta_returns_false() {
        // 候选段 s=Up，但 δ=Long 要求 s=Down。
        let context = vec![
            down_seg(50, 100, 0, 4), // s'（前序同向 Down，前提：需 s 方向 Down 才能比较）
            up_seg(60, 120, 5, 9),   // s=Up（方向不满足 δ=Long）
        ];
        let hist = flat_hist(1.0, 10);
        let input = DivCandInput { context: &context, target_idx: 1, hist: &hist, delta: Side::Long };
        assert!(!div_cand(&input), "方向不反 ⟹ Cand=0");
    }

    /// δ=Short 时，候选段方向必须为 Up；若方向为 Down ⟹ Cand=0。
    #[test]
    fn cond1_wrong_dir_short_delta_returns_false() {
        let context = vec![
            up_seg(50, 100, 0, 4),
            down_seg(40, 90, 5, 9), // s=Down，δ=Short 要求 Up
        ];
        let hist = flat_hist(1.0, 10);
        let input = DivCandInput { context: &context, target_idx: 1, hist: &hist, delta: Side::Short };
        assert!(!div_cand(&input), "方向不反（Short × Down）⟹ Cand=0");
    }

    // ── 条件2：Comparable（存在前序同向段） ───────────────────────────────────

    /// 无前序同向段（target=首段）⟹ Cand=0。
    #[test]
    fn cond2_no_previous_same_dir_returns_false() {
        // target_idx=0：前序为空，无 s'。
        let context = vec![down_seg(50, 100, 0, 4)];
        let hist = flat_hist(1.0, 5);
        let input = DivCandInput { context: &context, target_idx: 0, hist: &hist, delta: Side::Long };
        assert!(!div_cand(&input), "无前序同向段 ⟹ Cand=0");
    }

    /// 前序全为反向段（无同向段）⟹ Cand=0。
    #[test]
    fn cond2_only_opposite_dir_prev_returns_false() {
        let context = vec![
            up_seg(50, 100, 0, 4), // 方向 Up，与 δ=Long 的候选段 Down 不同向
            up_seg(60, 110, 5, 9), // 同上，非同向
            down_seg(30, 90, 10, 14), // s，δ=Long 方向 Down 正确
        ];
        let hist = flat_hist(1.0, 15);
        let input = DivCandInput { context: &context, target_idx: 2, hist: &hist, delta: Side::Long };
        assert!(!div_cand(&input), "前序无同向段 ⟹ 条件2 不满足");
    }

    // ── 条件3：Extreme（价格极值更进） ───────────────────────────────────────

    /// δ=Long：候选段 lo 不低于前段 lo ⟹ Cand=0。
    #[test]
    fn cond3_lo_not_lower_long_delta_returns_false() {
        let context = vec![
            up_seg(50, 100, 0, 4),   // 反向段（中间段，形成结构）
            down_seg(40, 90, 5, 9),  // s'，lo=40
            up_seg(45, 95, 10, 14),  // 反向
            down_seg(45, 90, 15, 19), // s，lo=45 >= lo(s')=40 ⟹ 不满足 Extreme
        ];
        // 前序最近同向段 = context[1]（Down，lo=40）
        let hist = flat_hist(2.0, 20);
        // 设 area(s')=10，area(s)=8（力度满足），但 Extreme 不满足。
        let hist_adj: Vec<f64> = (0..20).map(|i| if i < 10 { 2.0 } else { 1.5 }).collect();
        let input = DivCandInput { context: &context, target_idx: 3, hist: &hist_adj, delta: Side::Long };
        assert!(!div_cand(&input), "lo 不更低 ⟹ 条件3 不满足");
    }

    /// δ=Short：候选段 hi 不高于前段 hi ⟹ Cand=0。
    #[test]
    fn cond3_hi_not_higher_short_delta_returns_false() {
        let context = vec![
            down_seg(40, 90, 0, 4),   // 反向
            up_seg(50, 100, 5, 9),    // s'，hi=100
            down_seg(45, 95, 10, 14), // 反向
            up_seg(55, 95, 15, 19),   // s，hi=95 <= hi(s')=100 ⟹ 不满足 Extreme
        ];
        let hist = flat_hist(2.0, 20);
        let input = DivCandInput { context: &context, target_idx: 3, hist: &hist, delta: Side::Short };
        assert!(!div_cand(&input), "hi 不更高 ⟹ 条件3 不满足");
    }

    // ── 条件4：Weak（MACD 面积衰减） ────────────────────────────────────────

    /// area(s) >= area(s') ⟹ 力度未衰减 ⟹ Cand=0。
    #[test]
    fn cond4_no_divergence_returns_false() {
        let context = vec![
            up_seg(50, 100, 0, 4),    // 反向
            down_seg(40, 90, 5, 9),   // s'，lo=40，area=5*2=10
            up_seg(45, 95, 10, 14),   // 反向
            down_seg(30, 85, 15, 19), // s，lo=30 < 40（Extreme ✓），area=5*3=15 > 10（不衰减）
        ];
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { 2.0 } else { 3.0 }).collect();
        let input = DivCandInput { context: &context, target_idx: 3, hist: &hist, delta: Side::Long };
        assert!(!div_cand(&input), "area(s) > area(s') ⟹ 条件4 不满足");
    }

    // ── 四条件全满足（正例）──────────────────────────────────────────────────

    /// δ=Long：全部四条件满足 ⟹ Cand=1。
    #[test]
    fn all_four_conditions_satisfied_long_returns_true() {
        // 上涨 → 下跌(s') → 上涨 → 下跌(s)
        // s.lo(30) < s'.lo(40) [Extreme✓]；area(s)=5 < area(s')=10 [Weak✓]
        let context = vec![
            up_seg(50, 100, 0, 4),    // 反向
            down_seg(40, 90, 5, 9),   // s'：Down，lo=40，area=5*2=10
            up_seg(45, 95, 10, 14),   // 反向
            down_seg(30, 85, 15, 19), // s：Down，lo=30（< 40），area=5*1=5（< 10）
        ];
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { 2.0 } else { 1.0 }).collect();
        let input = DivCandInput { context: &context, target_idx: 3, hist: &hist, delta: Side::Long };
        assert!(div_cand(&input), "四条件全满足 δ=Long ⟹ Cand=1");
    }

    /// δ=Short：全部四条件满足 ⟹ Cand=1。
    #[test]
    fn all_four_conditions_satisfied_short_returns_true() {
        // 下跌 → 上涨(s') → 下跌 → 上涨(s)
        // s.hi(120) > s'.hi(100) [Extreme✓]；area(s)=5 < area(s')=10 [Weak✓]
        let context = vec![
            down_seg(40, 90, 0, 4),   // 反向
            up_seg(50, 100, 5, 9),    // s'：Up，hi=100，area=10
            down_seg(45, 95, 10, 14), // 反向
            up_seg(55, 120, 15, 19),  // s：Up，hi=120（> 100），area=5
        ];
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { 2.0 } else { 1.0 }).collect();
        let input = DivCandInput { context: &context, target_idx: 3, hist: &hist, delta: Side::Short };
        assert!(div_cand(&input), "四条件全满足 δ=Short ⟹ Cand=1");
    }

    // ── 空/边界 ───────────────────────────────────────────────────────────────

    /// 空 context ⟹ false（越界保护）。
    #[test]
    fn empty_context_returns_false() {
        let input = DivCandInput {
            context: &[],
            target_idx: 0,
            hist: &[1.0],
            delta: Side::Long,
        };
        assert!(!div_cand(&input), "空 context ⟹ false");
    }

    /// target_idx 越界 ⟹ false。
    #[test]
    fn out_of_bounds_target_returns_false() {
        let context = vec![down_seg(40, 90, 0, 4)];
        let input = DivCandInput {
            context: &context,
            target_idx: 5,  // 越界
            hist: &flat_hist(1.0, 10),
            delta: Side::Long,
        };
        assert!(!div_cand(&input), "越界 target_idx ⟹ false");
    }

    /// hist 为空 ⟹ area=0.0 ⟹ 条件4 0 < 0 = false。
    #[test]
    fn empty_hist_area_zero_divergence_fails() {
        let context = vec![
            up_seg(50, 100, 0, 4),
            down_seg(40, 90, 5, 9),   // s'
            up_seg(45, 95, 10, 14),
            down_seg(30, 85, 15, 19), // s，Extreme ✓
        ];
        let input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &[], // 空 hist → area=0.0 → 0 < 0 = false
            delta: Side::Long,
        };
        assert!(!div_cand(&input), "hist 空 ⟹ area=0 ⟹ 条件4 不满足");
    }

    /// Cand=0 ⟹ NestRung.cand=false ⟹ NestCertificate.n_delta()=false。
    /// 验证 Cand 向上游传播为 NestCertificate 的 0 值（规格 N^δ：任一级 Cand=0 ⟹ 整体 0）。
    #[test]
    fn cand_false_propagates_to_n_delta_zero() {
        use super::super::nest::{NestCertificate, NestInterval, NestRung};
        use super::super::super::types::{BspBits, Side as CertSide};

        // Cand=false 场景（前序无同向段）。
        let context = vec![down_seg(50, 100, 0, 4)];
        let hist = flat_hist(1.0, 5);
        let input = DivCandInput { context: &context, target_idx: 0, hist: &hist, delta: CertSide::Long };
        let cand = div_cand(&input); // false
        assert!(!cand);

        // 喂入 NestCertificate：cand=false ⟹ n_delta()=false。
        let mut terminal = BspBits::default();
        terminal.buy1 = true; // 基例 Conf^+ = true
        let cert = NestCertificate {
            side: CertSide::Long,
            terminal,
            base_interval: NestInterval { end_time: 4, start_time: 0, idx: 0 },
            rungs: vec![NestRung {
                interval: NestInterval { end_time: 9, start_time: 0, idx: 0 },
                cand, // false
            }],
        };
        assert!(!cert.n_delta(), "任一级 Cand=false ⟹ n_delta()=false（N^δ 定义）");
    }

    /// Cand=true ⟹ NestCertificate 其他条件满足时 n_delta()=true（多级链正例）。
    #[test]
    fn cand_true_with_valid_chain_n_delta_true() {
        use super::super::nest::{NestCertificate, NestInterval, NestRung};
        use super::super::super::types::{BspBits, Side as CertSide};

        // Cand=true（四条件全满足）。
        let context = vec![
            up_seg(50, 100, 0, 4),
            down_seg(40, 90, 5, 9),   // s'
            up_seg(45, 95, 10, 14),
            down_seg(30, 85, 15, 19), // s
        ];
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { 2.0 } else { 1.0 }).collect();
        let input = DivCandInput { context: &context, target_idx: 3, hist: &hist, delta: CertSide::Long };
        let cand = div_cand(&input);
        assert!(cand, "前置：四条件满足 Cand=true");

        // 构造两级证书（执行级 e=0，操作级 ℓ=1）。
        let mut terminal = BspBits::default();
        terminal.buy1 = true;
        // 执行级区间 [15,19]，操作级区间 [0,19]（⊇ 执行级）。
        let base = NestInterval { end_time: 19, start_time: 15, idx: 0 };
        let op_rung = NestRung {
            interval: NestInterval { end_time: 19, start_time: 0, idx: 0 },
            cand,
        };
        let cert = NestCertificate {
            side: CertSide::Long,
            terminal,
            base_interval: base,
            rungs: vec![op_rung],
        };
        // is_sub(base, op_rung.interval)：[15,19] ⊆ [0,19] ✓。
        assert!(cert.n_delta(), "Cand=true + 区间套成立 + Conf^+ ⟹ n_delta()=true");
    }

    // ── bsp_div_cand：从塔定位候选段并计算 DivCand ──────────────────────────

    use super::super::recursive_tower::{ElementId, LeveledMove};
    use super::super::descend::RMove as TestRMove;
    use super::super::super::types::Center;

    /// 构建合成 LeveledMove（L0 Segment）。
    fn seg_move(dir: Direction, lo: i64, hi: i64, start: usize, end: usize, ord: u64) -> LeveledMove {
        use std::rc::Rc;
        let rmove = TestRMove::Segment { direction: dir, lo, hi };
        LeveledMove {
            rmove,
            start_index: start,
            end_index: end,
            sub_moves: Rc::new(Vec::new()),
            id: ElementId { level: 0, ordinal: ord },
        }
    }

    /// 构建合成 Compose LeveledMove（包含 sub_moves）。
    fn compose_move(subs: Vec<LeveledMove>, level: u32, ord: u64) -> LeveledMove {
        use std::rc::Rc;
        let start = subs.first().map(|m| m.start_index).unwrap_or(0);
        let end = subs.last().map(|m| m.end_index).unwrap_or(0);
        let sub_rmoves: Vec<_> = subs.iter().map(|m| m.rmove.clone()).collect();
        LeveledMove {
            rmove: TestRMove::Compose {
                subs: sub_rmoves,
                centers: vec![Center { zd: lo_of(&subs), zg: hi_of(&subs), dd: lo_of(&subs), gg: hi_of(&subs), start_index: start, end_index: end }],
                level,
            },
            start_index: start,
            end_index: end,
            sub_moves: Rc::new(subs),
            id: ElementId { level, ordinal: ord },
        }
    }

    fn lo_of(subs: &[LeveledMove]) -> i64 {
        subs.iter().map(|m| m.rmove.lo()).min().unwrap_or(0)
    }

    fn hi_of(subs: &[LeveledMove]) -> i64 {
        subs.iter().map(|m| m.rmove.hi()).max().unwrap_or(0)
    }

    /// bsp_div_cand：塔中无目标走势（level 不存在或 source_index 无匹配）⟹ false。
    #[test]
    fn bsp_div_cand_missing_target_returns_false() {
        use super::super::recursive_tower::LeveledMove;
        use std::rc::Rc;

        // 空塔。
        let tower: Vec<Rc<Vec<LeveledMove>>> = vec![];
        let hist = flat_hist(1.0, 10);
        // 函数不存在时这里会编译错误（RED）。
        assert!(!super::bsp_div_cand(&tower, 0, 5, Side::Long, &hist),
            "空塔 ⟹ false");
    }

    /// bsp_div_cand：塔中有父 Compose，但 sub_moves 中无前序同向段 ⟹ false（条件2 不满足）。
    #[test]
    fn bsp_div_cand_no_prev_same_dir_returns_false() {
        use std::rc::Rc;

        // L0：4段（up/down/up/down），目标段 source_index=19（最后一段 Down end=19）
        let s0 = seg_move(Direction::Up,   50, 100, 0,  4,  0);
        let s1 = seg_move(Direction::Down, 40,  90, 5,  9,  1);
        let s2 = seg_move(Direction::Up,   45,  95, 10, 14, 2);
        let s3 = seg_move(Direction::Down, 30,  85, 15, 19, 3); // target

        // L1：Compose（包含全部4个 L0 段）
        let parent = compose_move(vec![s0.clone(), s1.clone(), s2.clone(), s3.clone()], 1, 0);

        let tower: Vec<Rc<Vec<_>>> = vec![
            Rc::new(vec![s0, s1, s2, s3]),    // L0
            Rc::new(vec![parent]),             // L1
        ];
        // s3（Down）的前序同向段=s1（Down）→ 应有前序同向段；但 s3.lo=30 < s1.lo=40（Extreme ✓）。
        // 力度：若 hist 全 0 ⟹ area=0 ⟹ 0 < 0 = false（条件4 不满足）。
        let hist = flat_hist(0.0, 20);
        assert!(!super::bsp_div_cand(&tower, 0, 19, Side::Long, &hist),
            "hist=0 ⟹ 条件4 不满足 ⟹ false");
    }

    /// bsp_div_cand：四条件全满足 ⟹ true。
    #[test]
    fn bsp_div_cand_all_conditions_true() {
        use std::rc::Rc;

        // 同 all_four_conditions_satisfied_long_returns_true 的结构。
        let s0 = seg_move(Direction::Up,   50, 100,  0,  4, 0);
        let s1 = seg_move(Direction::Down, 40,  90,  5,  9, 1); // s'：Down，lo=40
        let s2 = seg_move(Direction::Up,   45,  95, 10, 14, 2);
        let s3 = seg_move(Direction::Down, 30,  85, 15, 19, 3); // s：Down，lo=30 < 40

        let parent = compose_move(vec![s0.clone(), s1.clone(), s2.clone(), s3.clone()], 1, 0);
        let tower: Vec<Rc<Vec<_>>> = vec![
            Rc::new(vec![s0, s1, s2, s3]),
            Rc::new(vec![parent]),
        ];
        // hist：s'(5-9) area=5*2=10，s(15-19) area=5*1=5 < 10（条件4 ✓）。
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { 2.0 } else { 1.0 }).collect();
        assert!(super::bsp_div_cand(&tower, 0, 19, Side::Long, &hist),
            "四条件全满足 ⟹ bsp_div_cand = true");
    }
}
