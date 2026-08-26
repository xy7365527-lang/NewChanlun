//! 盘整背驰证书与判定（#1176 B01 判据块自 `signal.rs` 迁出，零行为）。
//!
//! 承载 [`PanDivCert`] / [`QuasiSecondCert`] 证书类型、[`PanDivStructure`] 纯结构载体与
//! 盘整背驰判定（[`judge_pan_div`] / [`judge_pan_div_observation`]），以及类第二类点生产构造点
//! （[`judge_quasi_second`]，#1233 裁定 a）。消费面经 `signal` 重导出保持原路径。

use super::super::cand_predicate::rmove_dir;
use super::super::recursive_tower::{descend_leveled, find_move_containing_index, LeveledMove};
use super::super::rmove_compose::retrace_no_break;
use super::*;
use crate::theta_v0::parser::segment::segment_force_l;

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

/// 类第二类买卖点载体（027:68，构造点落地 = #1233 裁定 a）。
///
/// `027-第27课.md:68`【正文】：「类似的，在大级别里，如果不出现新低，但可以构成类似第二类
/// 买点的买点，在MACD上，显示出类似背驰时的表现，黄白线回拉0轴上下，而后一柱子面积小于
/// 前一柱子的。一个最典型的例子，就是季度图上的600685，2005年的第三季度的2.21元构成一个
/// 典型的类第二类买点。」命名名分：ADR 0001 补充十六——「类第一类后的回抽点 = **类第二类**
/// （027:68 命名），归盘背通道观测」（086:70「新走势的类第二类」系比喻用法，与 027:68 正式
/// 命名分清，同补充十六）。
///
/// 判据 = #1233 裁定 a 五要素（[`judge_quasi_second`]）：大级别盘整语境 + 不出现新低 +
/// 力度不背驰（ForceL，beichi.md #873 正本；MACD 面积只作对照档 #990 口径）+ 区间套定位。
/// 旧「判据归 #817 未裁」的标注已订正——#817 票面零命中、判据本无主（#1254 在案）。
///
/// **消费路径（载体待接）**：构造点 [`judge_quasi_second`] 已落地，但本证书**暂未接入**
/// `LevelState`/six-bit/BspPoint/生命周期/订单流——判据落地后由下游实施票随盘背通道接线
/// （同 [`PanDivCert`] → `pan_div` 先例），在此之前不得把本载体当信号消费。
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

/// 类第二类点生产构造点（#1233 裁定 a；027:68 五要素落地）。
///
/// ## 判据要素（#1233 裁定 a 五要素的第 1–4 项；第 5 项「旧注释订正」落 [`QuasiSecondCert`] 文档）
/// 1. **大级别盘整语境**：`quasi_first` 是盘整背驰证书（[`PanDivCert`]，类第一类），其存在本身
///    即「大级别盘整、无趋势背驰」语境的证明；调用方保证 `parent` 是它所属的大级别走势，本
///    构造点只做坐标包含的防御性检查；
/// 2. **不出现新低**（买侧；卖侧镜像不创新高）：结构谓词 [`retrace_no_break`]——比的是次级别
///    走势的结构极值（053:28 同族「不创新低/新高」口径），不裸用逐 bar 价格比较；
/// 3. **力度不背驰**：`L(回抽段) >= L(前导离开段)`（ForceL，beichi.md #873 正本，经 #989
///    [`segment_force_l`] 段→笔反查；MACD 面积只作对照档 #990 口径，**不进本判据**）；
/// 4. **区间套定位**：复用 [`descend_leveled`] 取回次级别走势序列 + [`find_move_containing_index`]
///    定位前导类第一类点所在段；回抽段 = 其后首个**同向**次级别走势（「逐步往下」由调用方逐级
///    descend 收敛，本构造点是单级定位——回抽段终点即候选点坐标）。
///
/// 任一判据不满足 ⟹ `None`（合法定位失败，非 bug）。无笔数据 ⟹ ForceL 无源不判（不降级）。
///
/// ## 与普通第二类的边界
/// 普通第二类（[`super::super::rmove_compose::find_second_type_structure`]）由**次级别第一类**
/// （破中枢 ∧ 背驰）构成，回拉破不破一类极值只作重合标注（#816 B-2②）；类第二类是**盘整背驰**
/// （类第一类）之后「不出现新低 + 力度不背驰」的回抽点（027:68 的弱形式），两条不并存
/// （收敛通则）。
// 载体待接（#1269 验收口径）：消费路径接线前本构造点无生产调用方；接线票落地时移除本 allow。
#[allow(dead_code)]
pub(crate) fn judge_quasi_second(
    parent: &LeveledMove,
    quasi_first: &PanDivCert,
    strokes: &[Stroke],
) -> Option<QuasiSecondCert> {
    let side = quasi_first.side;
    // 要素1（防御）：类第一类点坐标必须落在 parent 覆盖区间内。
    if quasi_first.source_index < parent.start_index || quasi_first.source_index > parent.end_index
    {
        return None;
    }
    // 要素4：descend 取回次级别走势序列（携坐标侧车）。
    let subs = descend_leveled(parent);
    let subs = subs.as_slice();
    // 前导类第一类点所在次级别走势（区间包含定位——departure 终点可落段内部，同 #1052/#1076）。
    let i1 = find_move_containing_index(subs, quasi_first.source_index)?;
    let m1 = &subs[i1];
    // 回抽段方向 = 离开段方向（Long 买侧 = Down 回抽 / Short 卖侧 = Up 回抽；pan_div_side_with_policy
    // 已把 side 与 C 段方向绑定）。
    let departure_dir = match side {
        Side::Long => Direction::Down,
        Side::Short => Direction::Up,
    };
    // 回抽段 = m1 之后首个同向次级别走势（053:28「高点一次级别向下后一次级别向上」同族：跳过
    // 反向反弹段）。
    let m2 = subs[i1 + 1..]
        .iter()
        .find(|m| rmove_dir(&m.rmove) == Some(departure_dir))?;
    // 要素2：不出现新低/新高（结构谓词）。
    if !retrace_no_break(side, &m1.rmove, &m2.rmove) {
        return None;
    }
    // 要素3：力度不背驰——L(回抽段) >= L(离开段)（ForceL；任一段无笔 ⟹ 无源不判）。
    let l_prev = segment_force_l(strokes, m1.start_index, m1.end_index);
    let l_cur = segment_force_l(strokes, m2.start_index, m2.end_index);
    if !matches!((l_cur, l_prev), (Some(lc), Some(lp)) if lc >= lp) {
        return None;
    }
    Some(QuasiSecondCert {
        source_index: m2.end_index,
        side,
        quasi_first_index: quasi_first.source_index,
        retrace: (m2.start_index, m2.end_index),
    })
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
/// 的最近同向段，061:26 正文「只要是围绕一中枢的两段走势都可以比较力度」（A′→中枢→C）；080:168「99开始的向上就要和93-96的形成
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
    // A′ = 中枢前最近同向段（061:26 正文「只要是围绕一中枢的两段走势都可以比较力度」；p113 Part B `prev_same.find(dir, c.start_index)`
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
///    间存在回中枢段）；窄锚不可得回退 **A′ = 中枢前最近同向段**（061:26 正文「只要是围绕一中枢的两段走势都可以比较力度」，回中枢
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

#[cfg(test)]
mod tests {
    use super::super::super::recursive_tower::ElementId;
    use super::*;
    use std::rc::Rc;

    /// 测试用次级别走势（L0 线段，sub_moves 空）。
    fn seg(direction: Direction, lo: Tick, hi: Tick, start: usize, end: usize) -> LeveledMove {
        LeveledMove {
            rmove: RMove::Segment { direction, lo, hi },
            start_index: start,
            end_index: end,
            sub_moves: Rc::new(vec![]),
            id: ElementId {
                level: 0,
                ordinal: 0,
            },
        }
    }

    /// 测试用笔（ForceL 数据源）。
    fn stroke(dir: Direction, start: usize, end: usize, sp: Tick, ep: Tick) -> Stroke {
        Stroke {
            direction: dir,
            start_index: start,
            end_index: end,
            start_price: sp,
            end_price: ep,
        }
    }

    /// 大级别走势（level 1，携三个次级别段）。
    fn parent(subs: Vec<LeveledMove>) -> LeveledMove {
        let centers = vec![Center {
            zd: 50,
            zg: 90,
            dd: 40,
            gg: 100,
            start_index: 0,
            end_index: 17,
        }];
        LeveledMove {
            rmove: RMove::Compose {
                subs: Rc::new(subs.iter().map(|m| m.rmove.clone()).collect()),
                centers,
                level: 1,
            },
            start_index: 0,
            end_index: 17,
            sub_moves: Rc::new(subs),
            id: ElementId {
                level: 1,
                ordinal: 0,
            },
        }
    }

    /// 前导类第一类点（盘整背驰证书；构造点只消费 source_index 与 side）。
    fn pan_div(source_index: usize, side: Side) -> PanDivCert {
        PanDivCert {
            source_index,
            side,
            center: Center {
                zd: 50,
                zg: 90,
                dd: 40,
                gg: 100,
                start_index: 0,
                end_index: 6,
            },
            seg_a: (0, 0),
            seg_c: (0, source_index),
        }
    }

    /// 买侧正例夹具：离开段 Down [0,6]（低 50）→ 反弹 Up [7,10] → 回抽 Down [11,17]（低 55，
    /// 不创新低）。笔速度：离开段 v=5→7.5（L=+2.5），回抽段 v=2.5→6.25（L=+3.75 ≥ +2.5）。
    fn long_fixture() -> (LeveledMove, Vec<Stroke>) {
        let subs = vec![
            seg(Direction::Down, 50, 100, 0, 6),
            seg(Direction::Up, 50, 90, 7, 10),
            seg(Direction::Down, 55, 90, 11, 17),
        ];
        let strokes = vec![
            stroke(Direction::Down, 0, 3, 100, 80),
            stroke(Direction::Down, 3, 6, 80, 50),
            stroke(Direction::Up, 7, 10, 50, 90),
            stroke(Direction::Down, 11, 14, 90, 80),
            stroke(Direction::Down, 14, 17, 80, 55),
        ];
        (parent(subs), strokes)
    }

    /// 卖侧正例夹具：离开段 Up [0,6]（高 100）→ 回拉 Down [7,10] → 回抽 Up [11,17]（高 98，
    /// 不创新高）。笔速度：离开段 v=6.25→6.25（L=0），回抽段 v=1→1（L=0 ≥ 0）。
    fn short_fixture() -> (LeveledMove, Vec<Stroke>) {
        let subs = vec![
            seg(Direction::Up, 50, 100, 0, 6),
            seg(Direction::Down, 90, 100, 7, 10),
            seg(Direction::Up, 90, 98, 11, 17),
        ];
        let strokes = vec![
            stroke(Direction::Up, 0, 3, 50, 75),
            stroke(Direction::Up, 3, 6, 75, 100),
            stroke(Direction::Down, 7, 10, 100, 90),
            stroke(Direction::Up, 11, 14, 90, 94),
            stroke(Direction::Up, 14, 17, 94, 98),
        ];
        (parent(subs), strokes)
    }

    /// 判据全过 ⟹ 产买侧类第二类证书。
    #[test]
    fn long_no_new_low_and_force_not_diverging_emits_cert() {
        let (p, strokes) = long_fixture();
        let cert = judge_quasi_second(&p, &pan_div(6, Side::Long), &strokes);
        assert_eq!(
            cert,
            Some(QuasiSecondCert {
                source_index: 17,
                side: Side::Long,
                quasi_first_index: 6,
                retrace: (11, 17),
            })
        );
    }

    /// 卖侧镜像（不创新高 + 力度不背驰）⟹ 产卖侧证书。
    #[test]
    fn short_no_new_high_and_force_not_diverging_emits_cert() {
        let (p, strokes) = short_fixture();
        let cert = judge_quasi_second(&p, &pan_div(6, Side::Short), &strokes);
        assert_eq!(
            cert,
            Some(QuasiSecondCert {
                source_index: 17,
                side: Side::Short,
                quasi_first_index: 6,
                retrace: (11, 17),
            })
        );
    }

    /// 要素2（不出现新低）失败：回抽段跌破离开段低点 ⟹ None。
    #[test]
    fn retrace_breaking_new_low_is_rejected() {
        let (_, strokes) = long_fixture();
        // 回抽段低点 45 < 离开段低点 50 ⟹ 创新低。
        let subs = vec![
            seg(Direction::Down, 50, 100, 0, 6),
            seg(Direction::Up, 50, 90, 7, 10),
            seg(Direction::Down, 45, 90, 11, 17),
        ];
        let p = parent(subs);
        assert_eq!(
            judge_quasi_second(&p, &pan_div(6, Side::Long), &strokes),
            None
        );
    }

    /// 要素3（力度不背驰）失败：回抽段 ForceL 衰减（L(回抽) < L(离开)）⟹ None。
    #[test]
    fn retrace_force_diverging_is_rejected() {
        // 回抽段不创新低（低 60 ≥ 50），但笔速度 v=3.75→3.75（L=0 < 离开段 L=+2.5）。
        let subs = vec![
            seg(Direction::Down, 50, 100, 0, 6),
            seg(Direction::Up, 50, 90, 7, 10),
            seg(Direction::Down, 60, 90, 11, 17),
        ];
        let strokes = vec![
            stroke(Direction::Down, 0, 3, 100, 80),
            stroke(Direction::Down, 3, 6, 80, 50),
            stroke(Direction::Up, 7, 10, 50, 90),
            stroke(Direction::Down, 11, 14, 90, 75),
            stroke(Direction::Down, 14, 17, 75, 60),
        ];
        let p = parent(subs);
        assert_eq!(
            judge_quasi_second(&p, &pan_div(6, Side::Long), &strokes),
            None
        );
    }

    /// 要素4（区间套定位）失败：离开段之后无同向回抽段 ⟹ None。
    #[test]
    fn no_same_direction_retrace_returns_none() {
        let subs = vec![
            seg(Direction::Down, 50, 100, 0, 6),
            seg(Direction::Up, 50, 90, 7, 10),
        ];
        let p = parent(subs);
        let strokes = vec![
            stroke(Direction::Down, 0, 3, 100, 80),
            stroke(Direction::Down, 3, 6, 80, 50),
            stroke(Direction::Up, 7, 10, 50, 90),
        ];
        assert_eq!(
            judge_quasi_second(&p, &pan_div(6, Side::Long), &strokes),
            None
        );
    }

    /// 要素3（无源不判）：无笔数据 ⟹ ForceL 无源 ⟹ None（不降级）。
    #[test]
    fn no_strokes_returns_none() {
        let (p, _) = long_fixture();
        assert_eq!(judge_quasi_second(&p, &pan_div(6, Side::Long), &[]), None);
    }

    /// 要素1（坐标包含防御）：类第一类点坐标不在 parent 覆盖区间内 ⟹ None。
    #[test]
    fn quasi_first_outside_parent_returns_none() {
        let (p, strokes) = long_fixture();
        assert_eq!(
            judge_quasi_second(&p, &pan_div(100, Side::Long), &strokes),
            None
        );
    }
}
