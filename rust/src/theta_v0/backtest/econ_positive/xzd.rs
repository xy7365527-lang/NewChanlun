//! 小转大确认凭据（#1175 A01 职责块自 `econ_positive.rs` 迁出，零行为）。
//!
//! 承载 [`XzdEvidence`] 与 `xzd_*`/`xiaozhuanda_confirm` 判据族。消费面经
//! `econ_positive` 重导出保持原路径。

use super::*;

/// 小转大确认凭据（次级别结构确认通道——独立于区间套；本级无背驰段可套 ⟹ **无 depth**）。
///
/// ## 结果包（六要素）
/// - **结论**：Type2/3 信号在下钻未锚定（[`DescendStop::NoDivergence`]，小转大域）时，用二类买卖点
///   代替区间套定位；门通行 = **每级同一个判据 C2∧C3(新中枢+突破)∧¬例外臂**（#1220 / #1204 裁定 a：
///   C3 从 level==1 扩到每级，消灭 #799/#804 宽严两档；例外臂 = 三买卖坐实后「强力不背驰创新高」，
///   43 课答疑唯一例外，力度按 #985 ForceL）。输出标注 `C2+C3(breakout) xzd`，不得沿用旧
///   `C2-only xzd` 标签。
/// - **定义依据**：`053:28`（二类点补充小转大）；第43课「背驰后新中枢+反向突破」原文语义；codex #44
///   终局裁定(c)（judge_third 归属链 vs last_zs 选择链结构性不重合，见 `.chanlun/review-results/
///   codex-decide-20260702-193853-5bbe.md`）。
/// - **边界条件**：若 L2 复测显示 level==1 子集 `c3_new_center_breakout_ok` 命中率为 0% 或 100%，
///   说明判据本身有实现问题（非死门/非全通过的真实结构应产生中间命中率），需回到 codex 复审，不得
///   静默接受（`acc_classification_level_hole_dx` 断言守护）。
/// - **下游推论**：Type2/3 小转大域从「证书 None 门直接拒」改为二通道分派；两通道输入域不相交
///   = Some/None 互斥（codex §6-4 同义反复，非经验命题）。#1220 起每级统一 C2∧C3 硬门，并加例外臂
///   （三买卖坐实后强力不背驰创新高 ⟹ 拒，43 课答疑唯一例外）。
/// - **谱系引用**：606（区间套有效域=Type1）、673（Cand^δ 三分拆）、知识库 L410（小转大补充定位）、
///   #44 探针（C3 center 匹配口径重设计裁决：结构性不重合）。
/// - **影响声明**：`gate_pass()` 改为 `type2_confirmed && c3_new_center_breakout_ok && !force_exception_ok`；
///   新增 `c3_new_center_exists`/`c3_new_center_breakout_ok`/`force_exception_ok` 三参门字段；旧 4 项 C3
///   死门诊断字段（`last_zs_exists`/`same_side_l0_type3_any`/`same_center_any`/`same_side_causal_ok`）
///   全部保留为诊断字段；不改 build_nest_certificate/n_delta。
///
/// **认识论 L0**：纯结构判据（新中枢存在性+突破几何）。C3 新判据的 level==1 命中率认识论等级见
/// `acc_classification_level_hole_dx` 测试（L2，真实 BTC 数据）。alpha 有效性待 W-VERIFY(#13)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct XzdEvidence {
    /// 本级信号精确点（bsp source_index）。
    pub source_index: usize,
    /// 本级 lvl（小转大域恒 ≥1：lvl==0 时 base gate 免门走区间套，不入本通道）。
    pub level: usize,
    /// δ 方向。
    pub side: Side,
    /// 判据观察点（as-of 首次塔=确认 bar，零前视）。codex §6-3：显式区分 source_index/confirm_index，
    /// C3「动态最后次级中枢」只用 confirm_index 及之前的塔快照（`cls_i` 因果分类），杜绝前视。
    pub confirm_index: usize,
    /// C2：本级二类买卖点成立（跨条目按 source_index 查同级 B2，codex §6-1）。**唯一参门项**。
    pub type2_confirmed: bool,
    /// C3（诊断字段，**不参与 gate_pass**——codex 终局裁定 §5.2：level>=2 结构性死门，level==1
    /// 需下方诊断字段另裁）：最后次级中枢出现三类买卖点（as-of 首次塔）。= `same_side_same_center`。
    pub sub_last_zs_type3: bool,
    /// 诊断（§5.4）：`s` 跨度内是否存在次级中枢（`last_zs` 是否非 None）。
    pub last_zs_exists: bool,
    /// 诊断（§5.4）：as-of 次级 bsp 中是否存在同向 Type3（不问 center 归属）。
    pub same_side_l0_type3_any: bool,
    /// 诊断（§5.4）：as-of 次级 bsp 中是否存在**任意侧** Type3 其 `center == last_zs`。
    pub same_center_any: bool,
    /// 诊断（§5.4）：同侧 Type3 候选中是否存在 `source_index <= confirm_index` 者——为 false 时
    /// 说明同侧 Type3 只存在于 confirm_index 之后（时间确认问题），而非 center 归属问题。
    pub same_side_causal_ok: bool,
    /// 诊断（§5.4）：次级 bsp（sub_bsp）中 Type3 点总数。原「lvl>=2 恒 0」死门前提
    /// （extract_second_for_level 只产 B2/S2）已被 codex-t1 裁定A + #123（0a35f0167c，级别≥1
    /// 一/三类候选生成实装，三类净增 2364）合法作废——lvl>=2 现可非 0，dx 死门改锁基线数值。
    pub sub_bsp_type3_count: usize,
    /// C3 新判据（codex #44 终局裁定(c)）：`source_index`~`confirm_index` 间是否存在新确认次级中枢。
    pub c3_new_center_exists: bool,
    /// C3 新判据（codex #44 终局裁定(c)）：新中枢是否被其后次级走势反向突破（第43课「背驰后新中枢+
    /// 反向突破」）。**level==1 硬门参门项**；level>=2 维持既定 C2-only——C3 脱 0 后经 codex #179 终裁
    /// （codex-xzd-grading-20260704，条件翻转/当前维持现状）：level>=2 的 C3 硬门有效域未证
    /// （breakout_ok=1/217=0.46% 近退化单点），非「判据在全 level 不适用」；翻转条件见终裁报告三条。
    pub c3_new_center_breakout_ok: bool,
    /// 例外臂（#1220 / #1204 裁定 3 / #985 ForceL）：三买卖坐实（[`Self::c3_new_center_breakout_ok`]）
    /// 之后，其后次级走势又以**强力度（不背驰）**向原方向创出新高/新低（43 课答疑 `043:194`
    /// 「除非出现强力不背驰创新高的情况」）。**参门项**：true ⟹ 三买卖坐实被推翻，回到「持有」
    /// 侧（拒）；力度判据 = #985 ForceL `L(C) >= L(B)`（[`xzd_force_exception`]，段→笔反查 #989）。
    pub force_exception_ok: bool,
}

impl XzdEvidence {
    /// 门通行（#1220 / #1204 裁定 a，坐实裁决接进出场下钻）：**每级同一个判据**
    /// `C2 ∧ C3(新中枢突破) ∧ ¬例外臂`——C3 从 level==1 扩到每级（消灭 #799/#804 宽严两档）；
    /// 例外臂（[`Self::force_exception_ok`]，三买卖坐实后强力不背驰创新高）成立 ⟹ 拒（43 课
    /// 答疑唯一例外，持有）。三卖未出（`c3_new_center_breakout_ok` 假）⟹ 中枢继续，拒。旧字段
    /// （`sub_last_zs_type3`/`same_side_l0_type3_any`/`same_center_any`/`same_side_causal_ok`）
    /// 全部保留为诊断字段，不参门。
    pub(in super::super) fn gate_pass(&self) -> bool {
        self.type2_confirmed && self.c3_new_center_breakout_ok && !self.force_exception_ok
    }
}

/// C2 跨条目查找（codex §6-1）：同级 bsp 列表按 source_index 找共生二类买卖点。
///
/// B1/B3（`extract_signals_with_hist`）与 B2（`extract_second_for_level`）在 mod.rs 是独立提取 +
/// extend + sort，**非按 source_index merge**。Type3-only 信号自身 `bits.buy2=false`，直读自身 bits
/// 拿不到共生 B2 ⟹ 必须显式查同级列表。匹配 δ 侧的 buy2/sell2。
///
/// 单源：改调 [`classifier::bsp::bsp_bit_at`]（issue #747 C1）——find-first 换 any 的族内独立
/// 成员（同锚点可多类点共存，不与 `bsp_at` 同构，不强并）。
pub(super) fn xzd_type2_confirmed(
    bsp_of_level: &[BspPoint],
    source_index: usize,
    side: Side,
) -> bool {
    crate::theta_v0::classifier::bsp::bsp_bit_at(bsp_of_level, source_index, |bits| match side {
        Side::Long => bits.buy2,
        Side::Short => bits.sell2,
    })
}

/// C3 as-of（codex §6-3；**诊断字段来源，不再参门**——codex 终局裁定 §5.2）：本级走势 `s` 的
/// **最后一个**次级中枢出现三类买卖点。
///
/// 次级别 = level lvl-1（小转大域 lvl≥1 恒成立）。用 as-of 首次塔（`cls_i` at confirm bar，因果无前视）。
/// 最后次级中枢 = `s` 跨度 [start,end] 内 end_index 最大的次级中枢（动态最后中枢，`044:56` as-of 口径）。
/// C3 = 存在次级三类 bsp 其 center 即该最后中枢（离开该中枢回试不入 ZG/ZD）。必要非充分（思维导图 128）。
/// 值等同 `XzdEvidence.sub_last_zs_type3` / `XzdC3Diag.same_side_same_center`。
pub(super) fn xzd_sub_last_zs_type3(
    s: &LeveledMove,
    sub_centers: &[Center],
    sub_bsp: &[BspPoint],
    side: Side,
) -> bool {
    let Some(last_zs) = sub_centers
        .iter()
        .filter(|c| c.start_index >= s.start_index && c.end_index <= s.end_index)
        .max_by_key(|c| c.end_index)
    else {
        return false; // s 内无次级中枢 ⟹ 无「最后次级中枢」⟹ C3 不成立
    };
    sub_bsp.iter().any(|q| {
        (match side {
            Side::Long => q.bits.buy3,
            Side::Short => q.bits.sell3,
        }) && q
            .center
            // #218 面 A 载体形态机械适配：三类点恒 Center 载体（退役判据语义不动）。
            .map_or(false, |o| matches!(o, crate::theta_v0::classifier::bsp::OwnerRef::Center(c) if c.start_index == last_zs.start_index && c.end_index == last_zs.end_index))
    })
}

/// C3 死门诊断分项（codex 终局裁定 §5.4；供 level==1 子集死门裁断消费——不参与 gate_pass）。
struct XzdC3Diag {
    last_zs_exists: bool,
    same_side_l0_type3_any: bool,
    same_center_any: bool,
    same_side_causal_ok: bool,
    sub_bsp_type3_count: usize,
}

/// 计算 [`XzdC3Diag`]（裁定 §5.4 精确规格）：`last_zs` 与 [`xzd_sub_last_zs_type3`] 用同一取法
/// （s 跨度内 end_index 最大的次级中枢），避免两条选择链再次分叉（呼应审计代理 §3.4 的开放疑点）。
fn xzd_c3_diag(
    s: &LeveledMove,
    sub_centers: &[Center],
    sub_bsp: &[BspPoint],
    side: Side,
    confirm_index: usize,
) -> XzdC3Diag {
    let last_zs = sub_centers
        .iter()
        .filter(|c| c.start_index >= s.start_index && c.end_index <= s.end_index)
        .max_by_key(|c| c.end_index);
    let is_same_side_type3 = |q: &BspPoint| match side {
        Side::Long => q.bits.buy3,
        Side::Short => q.bits.sell3,
    };
    let is_type3 = |q: &BspPoint| q.bits.buy3 || q.bits.sell3;
    XzdC3Diag {
        last_zs_exists: last_zs.is_some(),
        same_side_l0_type3_any: sub_bsp.iter().any(is_same_side_type3),
        same_center_any: match last_zs {
            Some(z) => sub_bsp.iter().any(|q| {
                is_type3(q)
                    && q.center.map_or(false, |o| matches!(o, crate::theta_v0::classifier::bsp::OwnerRef::Center(c) if c.start_index == z.start_index && c.end_index == z.end_index))
            }),
            None => false,
        },
        same_side_causal_ok: sub_bsp
            .iter()
            .any(|q| is_same_side_type3(q) && q.source_index <= confirm_index),
        sub_bsp_type3_count: sub_bsp.iter().filter(|q| is_type3(q)).count(),
    }
}

/// C3 死门诊断分项二：「背驰后新中枢+反向突破」（第43课语义，codex #44 终局裁定(c)）。
pub(super) struct XzdC3BreakoutDiag {
    /// `source_index`（背驰确认点）之后、`confirm_index` 之前是否存在已确认的次级中枢。
    pub(super) new_center_exists: bool,
    /// 该新中枢之后、`confirm_index` 之前的次级走势是否反向突破其核心区间（ZG/ZD）。
    pub(super) new_center_breakout_ok: bool,
}

/// C3 新判据（codex #44 终局裁定(c) 精确规格）：新中枢 = `sub_centers` 中
/// `start_index >= source_index && end_index <= confirm_index` 者（小转大 source 之后、confirm
/// 之前已确认的次级中枢）；突破 = 该中枢之后、confirm 之前的次级走势 `m` 满足
/// `Side::Long ⟹ m.rmove.hi() > z.zg`（向上破）/ `Side::Short ⟹ m.rmove.lo() < z.zd`（向下破）。
///
/// **off-by-one**：`start_index >= source_index` 是硬约束（不用 `>`）——中枢起点与 source 同 bar
/// 仍算「source 之后已确认」，最小单测钉住此边界（见 tests）。ZG/ZD 是最小结构突破口径；
/// GG/DD 属 C4，不混入本判据（裁定原文边界条件）。
pub(super) fn xzd_c3_new_center_breakout(
    source_index: usize,
    confirm_index: usize,
    side: Side,
    sub_centers: &[Center],
    sub_moves: &[LeveledMove],
) -> XzdC3BreakoutDiag {
    let new_centers: Vec<&Center> = sub_centers
        .iter()
        .filter(|c| c.start_index >= source_index && c.end_index <= confirm_index)
        .collect();
    let new_center_breakout_ok = new_centers.iter().any(|z| {
        sub_moves
            .iter()
            .filter(|m| m.start_index >= z.end_index && m.end_index <= confirm_index)
            .any(|m| match side {
                Side::Long => m.rmove.hi() > z.zg,
                Side::Short => m.rmove.lo() < z.zd,
            })
    });
    XzdC3BreakoutDiag {
        new_center_exists: !new_centers.is_empty(),
        new_center_breakout_ok,
    }
}

/// 例外臂力度原语（#1220 / #1204 裁定 3 / #985 ForceL）：三买卖坐实后「强力不背驰创新高/新低」。
///
/// 第43课答疑 `043-第43课.md:194`：「如果破了，那就一定要走，除非出现强力不背驰创新高的情况。」——
/// 三卖（三买）已出之后，若其后次级走势以**强力度（不背驰）**向原方向创出新高/新低，则三买卖
/// 坐实被推翻，回到「持有」侧（例外臂拒）。坐实窗口以候选 bar（`confirm_index`）为终点，无前瞻。
///
/// 判定（每级同一个判据，与 [`xzd_c3_new_center_breakout`] 同族反向）：
/// 1. **创新高/新低**（几何）：新中枢 `z` 被反向突破（三卖=跌破 `z.zd` / 三买=升破 `z.zg`）之后，
///    其后次级走势再以**原方向**突破 `z` 的另一边界（Short: `hi > z.zg` 创新高 / Long:
///    `lo < z.zd` 创新低）——「破前高」的结构代理 = 新中枢边界（#1204 裁定「反向突破=三买卖坐实」）。
/// 2. **不背驰**（力度）：该次级走势的 `L(段) >= L(前一同向次级走势)`（#985 ForceL，
///    `segment_force_l` 段→笔反查 #989）。任一段无笔 ⟹ 无源不判（不触发例外）。
///
/// 两条件同时成立才触发例外臂；只创新高但力度背驰（L 衰减）⟹ 不触发（放行侧，真三卖）。
pub(super) fn xzd_force_exception(
    source_index: usize,
    confirm_index: usize,
    side: Side,
    sub_centers: &[Center],
    sub_moves: &[LeveledMove],
    strokes: &[crate::theta_v0::types::Stroke],
) -> bool {
    use crate::theta_v0::classifier::cand_predicate::rmove_dir;
    use crate::theta_v0::parser::segment::segment_force_l;
    use crate::theta_v0::types::Direction;

    // 反向突破方向（三卖/三买）与例外方向（强力创新高/新低）互反。
    let (break_dir, exc_dir) = match side {
        Side::Short => (Direction::Down, Direction::Up),
        Side::Long => (Direction::Up, Direction::Down),
    };
    for z in sub_centers
        .iter()
        .filter(|c| c.start_index >= source_index && c.end_index <= confirm_index)
    {
        // 找第一个反向突破该新中枢的次级走势（与 c3_new_center_breakout 同一突破口径）。
        let Some(m_break) = sub_moves
            .iter()
            .filter(|m| m.start_index >= z.end_index && m.end_index <= confirm_index)
            .find(|m| {
                rmove_dir(&m.rmove) == Some(break_dir)
                    && match side {
                        Side::Short => m.rmove.lo() < z.zd,
                        Side::Long => m.rmove.hi() > z.zg,
                    }
            })
        else {
            continue;
        };
        // 其后次级走势中找「强力创新高/新低」。
        for m in sub_moves
            .iter()
            .filter(|m| m.start_index >= m_break.end_index && m.end_index <= confirm_index)
        {
            if rmove_dir(&m.rmove) != Some(exc_dir) {
                continue;
            }
            // 创新高/新低：突破新中枢另一边界（几何）。
            let new_extreme = match side {
                Side::Short => m.rmove.hi() > z.zg,
                Side::Long => m.rmove.lo() < z.zd,
            };
            if !new_extreme {
                continue;
            }
            // 不背驰：L(m) >= L(前一同向次级走势)（#985 ForceL）。
            let l_cur = segment_force_l(strokes, m.start_index, m.end_index);
            let l_prev = sub_moves
                .iter()
                .rev()
                .find(|p| p.end_index < m.start_index && rmove_dir(&p.rmove) == Some(exc_dir))
                .and_then(|p| segment_force_l(strokes, p.start_index, p.end_index));
            if let (Some(lc), Some(lp)) = (l_cur, l_prev) {
                if lc >= lp {
                    return true;
                }
            }
        }
    }
    false
}

/// C3 L1 零命中根因判别探针（codex #55 终局裁定(5) 精确规格）：**纯只读旁路**，区分「非重叠
/// 三段窗口压掉新中枢」（候选1，`detect_centers_with` 算法限制）vs「真实几何无新中枢」（候选2，
/// 市场事实）。不写 `Classification.levels[*].centers`，不改 `tower`，不改正常输出 digest；
/// 默认不调用（调用现场 `ECON_C3_OVERLAP_PROBE` 环境变量门控，生产/默认测试路径零开销）。
pub(super) struct XzdC3OverlapProbeDiag {
    /// 滑动一格扫描（非 `detect_centers_with` 的非重叠三段消费）产出的 post-source 新中枢窗口数
    /// （`start_index >= source_index && end_index <= confirm_index`，可能同一中枢被多个重叠窗口
    /// 重复命中——诊断计数，非去重集合）。
    pub(super) overlapping_new_center_count: usize,
    /// 上述新中枢中，被其后次级走势反向突破者的窗口数（突破规则完全复用
    /// [`xzd_c3_new_center_breakout`]：`Side::Long ⟹ hi>zg` / `Side::Short ⟹ lo<zd`）。
    pub(super) overlapping_breakout_count: usize,
    /// 第一个 post-source 新中枢的 `(start_index, end_index)`（调试用，无命中为 `None`）。
    pub(super) first_overlapping_center: Option<(usize, usize)>,
}

/// 滑动重叠窗口扫描（`sub_units.windows(3)`，步长1）——与生产 `detect_centers_with`（成立支+3、
/// 不成立支+1，非重叠）唯一的差异点。复用同一中枢构造函数 `center_from_segments`（方向交替+全三段
/// 核心非空，L0 完整判据——本探针只在 `lvl==1` 调用，`sub_units` 恒为 L0 段，`center_from_window`
/// 的几何路径不适用于此层）+ 同一突破规则（`xzd_c3_new_center_breakout`）。
pub(super) fn xzd_c3_overlap_window_probe(
    source_index: usize,
    confirm_index: usize,
    side: Side,
    sub_units: &[UnitRange],
    sub_moves: &[LeveledMove],
) -> XzdC3OverlapProbeDiag {
    let mut new_centers: Vec<Center> = Vec::new();
    for w in sub_units.windows(3) {
        if let Some(c) = center_from_segments(&w[0], &w[1], &w[2]) {
            if c.start_index >= source_index && c.end_index <= confirm_index {
                new_centers.push(c);
            }
        }
    }
    let first_overlapping_center = new_centers.first().map(|c| (c.start_index, c.end_index));
    let overlapping_breakout_count = new_centers
        .iter()
        .filter(|z| {
            sub_moves
                .iter()
                .filter(|m| m.start_index >= z.end_index && m.end_index <= confirm_index)
                .any(|m| match side {
                    Side::Long => m.rmove.hi() > z.zg,
                    Side::Short => m.rmove.lo() < z.zd,
                })
        })
        .count();
    XzdC3OverlapProbeDiag {
        overlapping_new_center_count: new_centers.len(),
        overlapping_breakout_count,
        first_overlapping_center,
    }
}

/// 从 L0 层携坐标走势塔（`tower[0]`，恒为 `RMove::Segment` 变体——递归底）还原 `UnitRange` 序列，
/// 供 [`xzd_c3_overlap_window_probe`] 的滑窗输入。**必须**取真实线段方向（`RMove::Segment.direction`），
/// 不能用 `recursive_tower::project_to_units` 的外缘折叠方向（`fold_direction` 是几何路径占位，
/// L0 完整判据 `DirAlternates` 需要真方向——见 center.rs 诚实有效域声明）。`tower[0]` 本就是
/// `moves_tower = units.iter().map(LeveledMove::from_unit).collect()`（mod.rs `classify_impl`
/// level0 处理前的初始塔）的逐字段包装，故此还原与 `detect_centers_with` 原本消费的 L0 段账本
/// bit-exact 一致（同 start/end/direction/lo/hi）。
pub(super) fn l0_units_from_tower(moves: &[LeveledMove]) -> Vec<UnitRange> {
    moves
        .iter()
        .map(|m| {
            let direction = match &m.rmove {
                RMove::Segment { direction, .. } => *direction,
                RMove::Compose { .. } => {
                    unreachable!(
                        "tower[0] 恒为 L0 RMove::Segment（递归底，调用侧只在 lvl==1 用本函数）"
                    )
                }
            };
            UnitRange {
                start_index: m.start_index,
                end_index: m.end_index,
                direction,
                lo: m.rmove.lo(),
                hi: m.rmove.hi(),
            }
        })
        .collect()
}

/// 小转大确认（设计 §2.2 C1∧C2；C3 改为「新中枢+突破」硬门——#1220 起每级同一个判据，
/// 门通行判据见 [`XzdEvidence::gate_pass`]）。
///
/// 前提（调用侧路由保证）：`s` 是执行级 tower[lvl] 中 end_index==source_index 的候选段，且信号已判为
/// 小转大域（Type2/3 ∧ 下钻未锚定 [`DescendStop::NoDivergence`] ⟹ build_nest_certificate 返回 None）。
/// C1（下钻未锚定）由调用侧保证，本函数不重判。evidence 始终构造（含 C2/C3/例外臂/诊断分项取值）——
/// 诊断可读分项，门读 gate_pass。`strokes` = 本 bar 因果前缀笔序列（#985 ForceL 例外臂数据源）。
#[allow(clippy::too_many_arguments)]
pub(super) fn xiaozhuanda_confirm(
    s: &LeveledMove,
    source_index: usize,
    confirm_index: usize,
    lvl: usize,
    side: Side,
    bsp_of_level: &[BspPoint],
    sub_centers: &[Center],
    sub_bsp: &[BspPoint],
    sub_moves: &[LeveledMove],
    strokes: &[crate::theta_v0::types::Stroke],
) -> XzdEvidence {
    let diag = xzd_c3_diag(s, sub_centers, sub_bsp, side, confirm_index);
    let breakout =
        xzd_c3_new_center_breakout(source_index, confirm_index, side, sub_centers, sub_moves);
    XzdEvidence {
        source_index,
        level: lvl,
        side,
        confirm_index,
        type2_confirmed: xzd_type2_confirmed(bsp_of_level, source_index, side),
        sub_last_zs_type3: xzd_sub_last_zs_type3(s, sub_centers, sub_bsp, side),
        last_zs_exists: diag.last_zs_exists,
        same_side_l0_type3_any: diag.same_side_l0_type3_any,
        same_center_any: diag.same_center_any,
        same_side_causal_ok: diag.same_side_causal_ok,
        sub_bsp_type3_count: diag.sub_bsp_type3_count,
        c3_new_center_exists: breakout.new_center_exists,
        c3_new_center_breakout_ok: breakout.new_center_breakout_ok,
        force_exception_ok: xzd_force_exception(
            source_index,
            confirm_index,
            side,
            sub_centers,
            sub_moves,
            strokes,
        ),
    }
}
