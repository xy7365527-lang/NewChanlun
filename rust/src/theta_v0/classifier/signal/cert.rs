//! 盘整背驰证书与判定（#1176 B01 判据块自 `signal.rs` 迁出，零行为）。
//!
//! 承载 [`PanDivCert`] / [`QuasiSecondCert`] 证书类型、[`PanDivStructure`] 纯结构载体与
//! 盘整背驰判定（[`judge_pan_div`] / [`judge_pan_div_observation`]）。消费面经 `signal`
//! 重导出保持原路径。

use super::*;

/// 盘整背驰证书（Q4 裁决，task #145：「盘整背驰不能消失——它必须被某级别买卖点或小转大/区间套
/// 证书承接」。`PanDiv^δ_ℓ ⟹ ∃e<ℓ, Conf^δ_e` 或 `PanDiv^δ_ℓ ⟹ XZD^δ_{ℓ↓e}`）。
///
/// **不是买卖点**：本证书**不置任何 six-bit、不产 BspPoint**——盘整背驰不冒充同级 B1/S1
/// （「标准第一类买卖点应锚定趋势背驰」，Do not call every consolidation divergence same-level
/// B1/S1. But do not drop it.）。承接路由在 econ 统计层（econ_positive::collect_signals 消费
/// `LevelState.pan_div`，走现有 Nest/XZD 二通道门；两门皆闭 ⟹ 诚实丢弃）。
///
/// 结构语义（第24课:34-36 + beichi.md:113 盘整背驰 = **同一中枢**两次同向离开，
/// `AbcDivergence.is_trend=false`）：
/// - `seg_c`：当前离开走势区间 I(C)（Q5 同款区间语义，source_index 闭区间）。既有生产承接支
///   末段破中枢核心；#483 的 C 不破核心支只复用本证书形状进入观测/诊断，不进入生产承接。
/// - `seg_a`：**前一次同向离开 episode 区间** I(A)（Q5 区间口径，A/C 对称）——锚段端点破核心，
///   与 C 之间存在回中枢段（否则是同一次离开）。
/// - Weak = MACD 面积 C < A（与 buy1 同一冻结力度原语 `segments_diverge`）。
///
/// **覆盖缺口（#885 S4-d 明写，验收口径）**：本证书的生产构造点 [`judge_pan_div`]（唯一生产
/// 构造点，`judge_segment` 的 Consolidation 块分支）**只覆盖「C 段破核心」一支**——
/// [`pan_div_side_with_policy`] 的生产调用恒 `allow_unbroken_c=false`；「C 端点严格位于核心
/// `(zd, zg)`」一支只存在于 #483 观测路径 [`judge_pan_div_observation`]，**不进**
/// `LevelState.pan_div`、不产 BspPoint、不进生命周期链。叠加 027:66「大级别（周线以上）盘整
/// 背驰构成类第一类买点」的准入判据未裁（**算不算、几段起算归 #817**），**任何经本证书统计
/// 的类一类点命中率都只是下界**。否则域归化域（037:18「按盘整背驰处理」一支）的生产可查
/// 载体 = [`super::super::LevelState::first_class_grades`]（#885 同票落地，与本证书并列、互不合并）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanDivCert {
    /// C 段端点 source_index（因果触发点；破核心支亦作承接路由定位键）。
    pub source_index: usize,
    /// 方向候选：向下 C = Long / 向上 C = Short。
    pub side: Side,
    /// 盘整背驰所在的中枢（A/C 两次离开的同一中枢 = B）。
    pub center: Center,
    /// I(A)（前一次同向离开 episode 区间，Q5 区间口径）source_index 闭区间。
    pub seg_a: (usize, usize),
    /// I(C)（当前离开走势区间）source_index 闭区间。
    pub seg_c: (usize, usize),
}

/// 类第二类买卖点载体（027:68，#885 S4-d）。
///
/// `027-第27课.md:68`【正文】：「类似的，在大级别里，如果不出现新低，但可以构成类似第二类
/// 买点的买点，在MACD上，显示出类似背驰时的表现，黄白线回拉0轴上下，而后一柱子面积小于
/// 前一柱子的。一个最典型的例子，就是季度图上的600685，2005年的第三季度的2.21元构成一个
/// 典型的类第二类买点。」命名名分：ADR 0001 补充十六——「类第一类后的回抽点 = **类第二类**
/// （027:68 命名），归盘背通道观测」（086:70「新走势的类第二类」系比喻用法，与 027:68 正式
/// 命名分清，同补充十六）。
///
/// **本票只建载体，不新定判据**：「算不算类第二类、回抽从哪起算、面积比较的几段口径」的
/// 准入判据归 [#817](https://github.com/xy7365527-lang/NewChanlun/issues/817)（未裁）。本类型
/// **暂无生产构造点**——判据落地前不得把本载体当信号消费（不置任何 six-bit、不产 BspPoint、
/// 不进生命周期/订单流，同 [`PanDivCert`] 的诚实缺省纪律）；构造点落地时记录随盘背通道进
/// `LevelState`（同 `pan_div` 先例），届时由 #817 实施票接线并补坐标查询。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuasiSecondCert {
    /// 回抽（回拉）结束点 source_index（候选点坐标；027:68「2.21元的相应区间的寻找，也是按
    /// 上面级别逐步往下找背驰段的方法实现」的落点）。
    pub source_index: usize,
    /// 方向：Long = 类第二类买点（类第一类买点之后回抽不创新低）/ Short = 镜像卖点。
    pub side: Side,
    /// 前导类第一类点（父母）的 source_index——027:68「不出现新低/新高」的参照极值所属点
    /// （ADR 0001 补充十六「类第一类后的回抽点」的父母身份）。
    pub quasi_first_index: usize,
    /// 回抽走势区间 source_index 闭区间（类第一类点之后到本点的回拉段）。
    pub retrace: (usize, usize),
}

/// 盘整背驰的纯结构 A/C 载体。
///
/// 与 [`PanDivCert`] 的边界刻意分开：本载体只证明同一盘整中枢内存在两次同向离开，
/// 并给出方向与 A/C 区间；它不消费 MACD 力度，也不置任何买卖点 bit。#92 的
/// `Cand = dir ∧ Comparable ∧ Extreme` provider 在此载体上另判 Extreme，力度判据仍留在
/// 背驰段确认层。旧 [`judge_pan_div`] 继续用同一结构定位后再判 Weak，行为不变。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PanDivStructure {
    pub source_index: usize,
    pub side: Side,
    pub center: Center,
    pub seg_a: (usize, usize),
    pub seg_c: (usize, usize),
}

/// `allow_unbroken_c` 仅为 #483 观测增加「C 端点严格位于核心 `(zd, zg)`」这一支；
/// 等号与反向越界仍拒绝，且该支仍须由 [`judge_pan_div_observation`] 的面积 `C<A` 门确认。
/// 其余方向、锚、区间和生产破核心判据均不变。
fn pan_div_side_with_policy(c: &Center, end: &SegEnd, allow_unbroken_c: bool) -> Option<Side> {
    let c_strictly_inside_core = allow_unbroken_c && c.zd < end.price && end.price < c.zg;
    match end.dir {
        Direction::Down if end.price < c.zd || c_strictly_inside_core => Some(Side::Long),
        Direction::Up if end.price > c.zg || c_strictly_inside_core => Some(Side::Short),
        _ => None,
    }
}

/// 只定位盘整 A/C 结构，不消费力度。
///
/// 调用前提与 [`judge_pan_div`] 相同。返回 Some 只表示方向与 Comparable 成立；#92 provider
/// 还必须调用 [`pan_div_structure_extreme`]，不得把本函数的 Some 直接命名为 Cand。
pub(crate) fn locate_pan_div_structure(
    c: &Center,
    seg: &Segment,
    segments: &[Segment],
    anchors_self: &[Option<Direction>],
) -> Option<PanDivStructure> {
    locate_pan_div_structure_with_policy(c, seg, segments, anchors_self, false)
}

/// #483 观测专用定位：复用既有窄锚机制，但允许 C 端点严格落在中枢核心 `(zd, zg)` 内。
///
/// 只供 [`judge_pan_div_observation`]；Consolidation Nest 与生产 [`judge_pan_div`] 继续调用
/// [`locate_pan_div_structure`]，因此不会把新分支送入生产决策或生命周期链。
fn locate_pan_div_structure_allowing_unbroken_c(
    c: &Center,
    seg: &Segment,
    segments: &[Segment],
    anchors_self: &[Option<Direction>],
) -> Option<PanDivStructure> {
    locate_pan_div_structure_with_policy(c, seg, segments, anchors_self, true)
}

fn locate_pan_div_structure_with_policy(
    c: &Center,
    seg: &Segment,
    segments: &[Segment],
    anchors_self: &[Option<Direction>],
    allow_unbroken_c: bool,
) -> Option<PanDivStructure> {
    let end = seg_end(seg);
    let side = pan_div_side_with_policy(c, &end, allow_unbroken_c)?;
    let dir = end.dir;
    let lo = segments.partition_point(|s| s.start_index < c.end_index);
    let hi = segments.partition_point(|s| s.start_index <= seg.start_index);
    let win = &segments[lo..hi];
    let reenters = |s: &Segment| {
        s.direction != dir
            && match dir {
                Direction::Down => s.end_price >= c.zd,
                Direction::Up => s.end_price <= c.zg,
            }
    };
    win.iter()
        .rev()
        .filter(|s| s.end_index <= seg.start_index)
        .find(|s| reenters(s))?;
    let lambda_c = departure_move_c_start(segments, anchors_self, c, dir, seg.start_index)?;
    let a_anchor = win
        .iter()
        .rev()
        .filter(|s| s.direction == dir && s.end_index <= lambda_c)
        .find(|s| match dir {
            Direction::Down => s.end_price < c.zd,
            Direction::Up => s.end_price > c.zg,
        })?;
    if !win
        .iter()
        .any(|s| reenters(s) && s.start_index >= a_anchor.end_index && s.end_index <= lambda_c)
    {
        return None;
    }
    let lambda_a = departure_move_c_start(segments, anchors_self, c, dir, a_anchor.start_index)?;
    let episode_end = win
        .iter()
        .find(|s| reenters(s) && s.start_index >= a_anchor.end_index)
        .map_or(lambda_c, |r| r.start_index);
    let rho_a = win
        .iter()
        .rev()
        .filter(|s| s.direction == dir && s.start_index >= lambda_a && s.end_index <= episode_end)
        .map(|s| s.end_index)
        .next()?;
    Some(PanDivStructure {
        source_index: end.source_index,
        side,
        center: *c,
        seg_a: (lambda_a, rho_a),
        seg_c: (lambda_c, seg.end_index),
    })
}

/// ★R3（2026-07-17 代理裁定，p113 Part B 实证 + doc-pan §6.2-1）：A 锚扩展——当中枢后找不到
/// 「前次同向破核心段」作 A（窄锚）时，候选 A′ = **中枢前最近同向段**（`end_index ≤ c.start_index`
/// 的最近同向段，061:28 中枢两头比较形态 A′→中枢→C；080:168「99开始的向上就要和93-96的形成
/// 盘整背驰」；049:36-38「最标准」≠唯一；080:364「不是光比较最近这一段的」）。
///
/// 与窄锚的结构差异：回中枢要件由中枢本身满足（A′ 与 C 之间隔着整个中枢），不再要求
/// 中枢后存在回中枢段；C 仍取当前离开 episode（λ_C 与窄锚同一 helper）。p113 实测：1,238 条
/// Cons-leave 中 99.6% 块（228/229）经本扩展可重新定位——A 锚窄化是第一击杀机制。
/// 调用顺序约定：先窄锚 [`locate_pan_div_structure`]，失败再回退本函数（窄锚是标准锚，
/// 049:36-38「最标准的情况当然是前面最近向下的」优先）。
pub(crate) fn locate_pan_div_structure_front_anchor(
    c: &Center,
    seg: &Segment,
    segments: &[Segment],
    anchors_self: &[Option<Direction>],
) -> Option<PanDivStructure> {
    locate_pan_div_structure_front_anchor_with_policy(c, seg, segments, anchors_self, false)
}

/// #483 观测专用 A′ 回退：与既有中枢前最近同向段锚逐位同构，仅增加 C 严格在核心内这一支。
fn locate_pan_div_structure_front_anchor_allowing_unbroken_c(
    c: &Center,
    seg: &Segment,
    segments: &[Segment],
    anchors_self: &[Option<Direction>],
) -> Option<PanDivStructure> {
    locate_pan_div_structure_front_anchor_with_policy(c, seg, segments, anchors_self, true)
}

fn locate_pan_div_structure_front_anchor_with_policy(
    c: &Center,
    seg: &Segment,
    segments: &[Segment],
    anchors_self: &[Option<Direction>],
    allow_unbroken_c: bool,
) -> Option<PanDivStructure> {
    let end = seg_end(seg);
    let side = pan_div_side_with_policy(c, &end, allow_unbroken_c)?;
    let dir = end.dir;
    let lambda_c = departure_move_c_start(segments, anchors_self, c, dir, seg.start_index)?;
    // A′ = 中枢前最近同向段（061:28 中枢两头比较；p113 Part B `prev_same.find(dir, c.start_index)`
    // 同口径：`end_index ≤ c.start_index` 的最近同向段）。
    let a_prime = segments
        .iter()
        .rev()
        .find(|s| s.direction == dir && s.end_index <= c.start_index)?;
    Some(PanDivStructure {
        source_index: end.source_index,
        side,
        center: *c,
        seg_a: (a_prime.start_index, a_prime.end_index),
        seg_c: (lambda_c, seg.end_index),
    })
}

/// #92 Cand 的 Extreme 分量；不消费 MACD 力度。
pub(crate) fn pan_div_structure_extreme(structure: &PanDivStructure, segments: &[Segment]) -> bool {
    let envelope = |span: (usize, usize)| {
        segments
            .iter()
            .filter(|segment| segment.start_index >= span.0 && segment.end_index <= span.1)
            .fold(None, |acc: Option<(Tick, Tick)>, segment| {
                let lo = segment.start_price.min(segment.end_price);
                let hi = segment.start_price.max(segment.end_price);
                Some(match acc {
                    None => (lo, hi),
                    Some((old_lo, old_hi)) => (old_lo.min(lo), old_hi.max(hi)),
                })
            })
    };
    let (Some(a), Some(c)) = (envelope(structure.seg_a), envelope(structure.seg_c)) else {
        return false;
    };
    match structure.side {
        Side::Long => c.0 < a.0,
        Side::Short => c.1 > a.1,
    }
}

/// 盘整背驰判定（Q4，[`PanDivCert`] 的唯一构造点）。
///
/// 前提（调用方保证）：`c` 是 `seg` 的最近已确认中枢，且 c 按 ownership 落在**本级别**
/// **Consolidation 块**（[`center_block_kind`] + `level_lift == 0`——#898：扩展折出的高一级
/// 盘整块不走本级盘背；与趋势门同一 decompose 单一来源）。判据链：
/// 1. **破中枢核心**（因果触发 = seg 端点，等价性同 judge_first_cached）：Down ⟹ 端点 < c.zd
///    （Long 候选）/ Up ⟹ 端点 > c.zg（Short）。
/// 2. **当前离开区间 I(C)**（Q5 区间语义）：λ_C = 最后一个回中枢段 r（反向段、端点回到核心内侧：
///    Down 破侧 end ≥ zd / Up 破侧 end ≤ zg，r 在 seg 之前）之后的首个同向段起点；I(C)=[λ_C, seg.end]。
/// 3. **A 锚**（★R3 2026-07-17 裁定）：先窄锚——同一中枢的前一次同向离开末段（端点破核心 ∧ A、C
///    间存在回中枢段）；窄锚不可得回退 **A′ = 中枢前最近同向段**（061:28 中枢两头比较，回中枢
///    要件由中枢本身满足）。窄锚定位成功即不再回退（最标准锚优先，049:36-38）。
/// 4. **Weak**（★R2 2026-07-17 裁定）：力度或关系——同色柱面积 C<A（060:44）∨ 黄白线峰 C<A
///    （026:521）∨ 同向柱峰 C<A（025:38），任一成立即背驰信号（027:32「只要其中一个符合就
///    可以」）。替代旧面积单通道必要门（混合柱 Σ|hist| 严格 curr<prev，p113 实测 30.4% 聋度）。
///
/// 无 A/A′ / 区间无法映射 closes / 或关系全无衰减 ⟹ None（033:26 无衰减即无盘背，诚实不产证书）。
/// 复杂度：窗口 = start_index ∈ [c.end_index, seg.start_index] 的段（二分定界 + 窗口内线性扫）。
pub(crate) fn judge_pan_div(
    c: &Center,
    seg: &Segment,
    segments: &[Segment],
    anchors_self: &[Option<Direction>],
    hist: &[f64],
    dif: &[f64],
    src_to_idx: &[usize],
) -> Option<PanDivCert> {
    let structure = locate_pan_div_structure(c, seg, segments, anchors_self)
        .or_else(|| locate_pan_div_structure_front_anchor(c, seg, segments, anchors_self))?;
    // 4. Weak：力度或关系（027:32/026:521/025:38；is_trend=false = 盘整背驰语义）。
    let (c_span, a_span) = (structure.seg_c, structure.seg_a);
    let (Some(c_idx), Some(a_idx)) = (
        map_src_range_to_close_idx(src_to_idx, c_span.0, c_span.1),
        map_src_range_to_close_idx(src_to_idx, a_span.0, a_span.1),
    ) else {
        return None; // 区间无法映射 closes ⟹ 无力度可读 ⟹ 不冒充背驰。
    };
    if !segments_diverge_or(hist, dif, structure.side, a_idx, c_idx) {
        return None; // 各 proxy 全无衰减 ⟹ 市场事实（033:26）⟹ 非盘整背驰。
    }
    Some(PanDivCert {
        source_index: structure.source_index,
        side: structure.side,
        center: structure.center,
        seg_a: a_span,
        seg_c: c_span,
    })
}

/// #483 盘整背驰观测判定。
///
/// - C 破核心：逐字调用既有 [`judge_pan_div`]，原 OR 力度判据与证书字段不变；
/// - C 不破核心：C 端点必须严格位于核心 `(zd, zg)`（等号拒绝），沿用同一窄锚→A′ 回退和
///   source→MACD 区间映射，并严格只认第24课“同色柱面积 C<A”；
/// - 返回既有 [`PanDivCert`] 形状，只供 `pan_div_diag` 观测。生产信号提取仍调用
///   [`judge_pan_div`]，因此新分支不进入 `LevelState.pan_div`、BspPoint 或生命周期链。
pub(crate) fn judge_pan_div_observation(
    c: &Center,
    seg: &Segment,
    segments: &[Segment],
    anchors_self: &[Option<Direction>],
    hist: &[f64],
    dif: &[f64],
    src_to_idx: &[usize],
) -> Option<PanDivCert> {
    let end = seg_end(seg);
    let c_breaks_core = match end.dir {
        Direction::Down => end.price < c.zd,
        Direction::Up => end.price > c.zg,
    };
    if c_breaks_core {
        return judge_pan_div(c, seg, segments, anchors_self, hist, dif, src_to_idx);
    }

    let structure = locate_pan_div_structure_allowing_unbroken_c(c, seg, segments, anchors_self)
        .or_else(|| {
            locate_pan_div_structure_front_anchor_allowing_unbroken_c(
                c,
                seg,
                segments,
                anchors_self,
            )
        })?;
    let (c_span, a_span) = (structure.seg_c, structure.seg_a);
    let (Some(c_idx), Some(a_idx)) = (
        map_src_range_to_close_idx(src_to_idx, c_span.0, c_span.1),
        map_src_range_to_close_idx(src_to_idx, a_span.0, a_span.1),
    ) else {
        return None;
    };
    if divergence::same_color_area(hist, c_idx.0, c_idx.1, structure.side)
        >= divergence::same_color_area(hist, a_idx.0, a_idx.1, structure.side)
    {
        return None;
    }
    Some(PanDivCert {
        source_index: structure.source_index,
        side: structure.side,
        center: structure.center,
        seg_a: a_span,
        seg_c: c_span,
    })
}
