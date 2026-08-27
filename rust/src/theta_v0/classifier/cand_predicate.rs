//! DivCand^δ_{Θ,ℓ}(s,t)：背驰段候选谓词（Cand 判据接入，W1 工位）。
//!
//! ## 定义（三方交叉确认：Lead 推导 + codex C1 裁决 + ChatGPT 推导一致）
//!
//! `DivCand^δ_{Θ,ℓ}(s,t)` :=
//!   [dir(s) = −δ]                             ← 条件1：方向反（δ=交易方向，背驰段走势方向）
//!   ∧ [∃s'∈S^vis: Comparable_ℓ(s',s,t)]      ← 条件2：存在同上级语境可比较前段 s'
//!   ∧ [Extreme^δ(s',s,t)]                     ← 条件3：价格极值更进一步
//!   ∧ [Weak^δ_{Θ,ℓ}(s,s',t)]                 ← 条件4：力度衰减（Weak = Diverge；★#883 起 =
//!                                                #990 统一判据原语，默认 ForceL 教义档）
//!
//! ## 操作化
//!
//! 给定上级走势（Compose）的次级别走势序列 `context`（来自 `LeveledMove.sub_moves`）和目标
//! 段在序列中的索引 `target_idx`：
//!
//! - **条件1**：`rmove_dir(context[target_idx])` 与 δ 方向相反
//!   - δ=Long(买)  →   dir(s) = Down  （下跌段末端背驰买）
//!   - δ=Short(卖) →   dir(s) = Up    （上涨段末端背驰卖）
//! - **条件2**（★#883 S4-b 重写：#814 D-3 统一取段规则，#979 裁定一/二）：以父走势最近中枢
//!   `c` 为「界」，s 自身须跨界（冲出核心），s' = 往回最近的**同方向跨界段**（首次离开 =
//!   进入段；反复震荡 = 上一次同向离开段；中枢内震荡段不参与）。趋势背驰 c vs b 是该规则
//!   在首次离开时的特例（b = 最后中枢 B 的进入段）——**盘整背驰入口即此，不再缺半壁**。
//!   旧口径「同父前序最近同向段」（无中枢、趋势形状）已退役，不并存（收敛通则）。
//! - **条件3**：
//!   - δ=Long：  `lo(s) < lo(s')`  （s 低点更低——下跌更深）
//!   - δ=Short：`hi(s) > hi(s')`  （s 高点更高——上涨更高）
//! - **条件4**（★#883：#990 收编后统一判据原语）：默认 `ForceL` 教义判据 `L(C)<L(B)`
//!   （#873 力度=速度净增量，经 #989 `segment_force_l` 段→笔反查）；`MacdArea` 同色柱面积
//!   （[`super::divergence::segment_macd_area`] + [`super::divergence::is_divergence`]）降为
//!   显式对照档。判定点 = [`super::divergence::confirm_divergence_l`] 单一原语。
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
//!   条件1 参照方向定义（δ=交易方向，背驰段方向 = −δ）；★#883 起：条件2 = #814 D-3 统一取段
//!   规则（#979 裁定一/二：趋势 c vs b 与盘整 C vs A 同一条规则）；条件4 = #990 收编后统一
//!   判据原语（默认 ForceL `L(C)<L(B)`，#873 教义经 #989 段→笔反查；MacdArea 为对照档）。
//! - **边界条件**：(1) `context.len() < 2` ⟹ false（无前段可比较）；
//!   (2) 无父中枢语境 / s 未跨界 / 前序无同向跨界段 ⟹ false（条件2 不满足，#883 D-3 取段）；
//!   (3) MACD hist 为空 ⟹ area=0.0 ⟹ 0 < 0 = false（MacdArea 档条件4 不满足）；strokes 为空
//!   或段区间无笔 ⟹ ForceL 档无源不判（条件4 不满足）；
//!   (4) 方向翻转（δ Long↔Short）⟹ 条件1 方向判定翻转；
//!   (5) 力度档经 `gauge` 显式切换（生产默认 ForceL，#990）。
//! - **下游推论**：Cand=true ⟹ NestRung.cand=true ⟹ NestCertificate.n_delta() 可为 true；
//!   Cand=false ⟹ n_delta()=false（spec N^δ 定义，任一级 Cand=0 ⟹ 整体 0）。
//! - **谱系引用**：W1 工位规格（三方一致，2026-07-01）；codex C1 裁决。不确定是否有相关
//!   概念分离谱系，保守声明。
//! - **影响声明**：新建本文件；暴露 `div_cand`/`div_cand_fail`/`bsp_div_cand`/`rmove_dir`/
//!   `parent_last_center` 供 `econ_positive.rs::build_multilevel_nest_cert`（W1 返工实际入口）
//!   调用；不改 divergence.rs / nest.rs / descend.rs；不碰识别层。L0 结构谓词。
//!   ★#883（S4-b）：`DivCandInput` 增 `strokes`/`parent_center`/`gauge` 三字段——盘整背驰入口
//!   落地（下钻找「一类点**或类一类点**」的半壁补齐）；`locate_pan_div_structure`（L0 证书层
//!   结构定位）按 #814 D-3 受影响清单**零改**（#990 已裁：「只定位不判力度」本就是结构/度量
//!   两层分离的正确形态）。★#1265（#1231 裁定 a）：D-3 取段收敛为全仓唯一原语
//!   [`d3_prev_crossing_anchor`]——div_cand 条件2 与 PanDiv 窄锚调同一函数，窄锚「回中枢要件」
//!   退役（#1262：五处原文查无依据）。

use std::rc::Rc;

use super::super::parser::segment::segment_force_l;
use super::super::types::{Center, Direction, Segment, Side, Stroke, Tick};
use super::descend::RMove;
use super::divergence::{confirm_divergence_l, is_divergence, segment_macd_area, DivergenceGauge};
use super::recursive_tower::{find_move_by_end_index, map_src_to_close_idx, LeveledMove};

/// δ 交易方向（Long=买/+1，Short=卖/−1）。
/// 复用 types::Side（与 NestCertificate.side 同类型）。
pub use super::super::types::Side as Delta;

/// DivCand^δ 单次判定所需上下文（从塔提取）。
///
/// `context`：候选段所在上级走势的次级别走势序列（`parent.sub_moves`）。
/// `target_idx`：候选段在 `context` 中的索引（`context[target_idx]` 是目标段 s）。
/// `hist`：MACD hist 序列（全 bar 域，bar 索引对齐）。
/// `delta`：交易方向 δ（Long=买候选找下跌背驰，Short=卖候选找上涨背驰）。
/// `strokes`（★#883 S4-b）：L0 笔序列（source_index 域，跨级同坐标系），供 ForceL 教义判据
/// `L(C)<L(B)`（#873，经 #989 `segment_force_l` 反查）；空 ⟹ ForceL 档无源不判（不降级——
/// 收敛通则禁宽松接管）。
/// `parent_center`（★#883 S4-b）：父走势（context 的所属 Compose）的**最近中枢**——#814 D-3
/// 统一取段规则的「界」。`None`（父无中枢）⟹ 跨界无定义 ⟹ 条件2 不可满足（合法定位失败，
/// 非 bug）；**不退回旧的「最近同向段」无中枢口径**（#979 裁定一：D-3 是唯一取段规则，趋势
/// c vs b 是它在首次离开时的特例，不并存第二套取段）。
/// `gauge`（★#883 S4-b）：力度判据档（#990 收编后统一判据原语，禁第二套力度引擎）。生产 =
/// `DivergenceGauge::default()`（ForceL）；`MacdArea` 为显式对照档（ADR-0005）。本谓词无
/// 5-proxy 源 ⟹ ThetaDom/ThetaLex/Conjunction 档恒不判（诚实，不 fallback）。
pub struct DivCandInput<'a> {
    /// 候选段所在上级走势的次级别走势序列（`parent.sub_moves` 切片，直读不投影）。
    pub context: &'a [LeveledMove],
    /// 目标段在 `context` 中的索引。
    pub target_idx: usize,
    /// MACD hist 序列（全 bar 域，从 bar 0 开始）。
    pub hist: &'a [f64],
    /// 交易方向 δ。
    pub delta: Delta,
    /// L0 笔序列（source_index 域），ForceL 力度原语的数据源。
    pub strokes: &'a [Stroke],
    /// 父走势最近中枢（D-3 取段的「界」）。
    pub parent_center: Option<&'a Center>,
    /// 力度判据档（生产默认 ForceL；MacdArea 对照）。
    pub gauge: DivergenceGauge,
    /// ★#1228：`hist` 坐标系映射——`Some(close_src)` 时 hist 是 merged-bar 下标域（`hist[k]` 对应
    /// `close_src[k]` 的 source_index），条件4 的 MACD 段面积须经
    /// [`super::recursive_tower::map_src_to_close_idx`] 把段的 source_index 区间映射到 hist 下标；
    /// `None` 时 hist 即 source_index 域（econ_positive 全量路径，直读）。ForceL 档不消费 hist
    /// （力度走 strokes 的 source_index 域），故本字段只影响 MacdArea 对照档的面积读数。
    pub close_src: Option<&'a [usize]>,
}

/// `RMove::Compose` 的中枢序列方向判据（#815 M-2 三个候选臂）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirCriterion {
    /// 核心分离：上行 `last.zd > first.zg`，下行 `last.zg < first.zd`。
    CoreSeparation,
    /// 外缘分离：上行 `last.dd > first.gg`，下行 `last.gg < first.dd`。
    EnvelopeSeparation,
    /// 双升双降（#815 底稿第三臂）：`zg`/`zd`（中枢上下沿）同向严格移动——底稿 Python
    /// `_centers_relation_by_zg_zd` 比较的 Center `high`/`low` 实为 ZG/ZD（#900 F1 对齐）。
    /// **变体名从 #815 三臂称呼保留**（「Envelope」不指外包络 GG/DD，指第三臂的命名史），
    /// 不重命名以免 #870 对照引用漂移。
    DualEnvelopeRiseFall,
}

impl Default for DirCriterion {
    fn default() -> Self {
        DEFAULT_DIR_CRITERION
    }
}

/// 当前生产押注的方向判据：外缘分离。
pub const DEFAULT_DIR_CRITERION: DirCriterion = DirCriterion::EnvelopeSeparation;

/// 走势方向判定：`Segment` 直读；`Compose` 按中枢序列判断。
///
/// **名分：`[旧缠论]`（2026-08-04，#815 M-2 裁定，依据 `020-第20课.md:58`）。**
/// 默认的**外缘分离**（上行 `last.dd > first.gg`，下行 `last.gg < first.dd`）**已转正**——
/// 中心定理二把两种边界写在同一句里并各自指派角色：`DD`/`GG` 管「是不是趋势」，
/// `ZD`/`ZG` 管「是不是扩展」。**核心分离但外缘仍重叠 ＝ 中枢扩展、升一级，不是趋势。**
/// 另两条候选臂（核心分离 `last.zd > first.zg` / `last.zg < first.zd`、双升双降）**保留**，
/// 仅作 #870 三臂重测的对照，**不再是待裁教义**。
///
/// ⚠️ 订正一处曾被引用的错误陈述：「三套判据接受集互不包含」**是错的**（源出 #815 原票面，
/// 引者未自核）。因构造保证 `DD <= ZD < ZG <= GG`（`center.rs:216-227`），三者是**严格链**。
///
/// 判据链接：#870 对 #846 的 301 条样本三臂各跑一遍的对照价值仍在（哪套让约 92% 的失败率
/// 降得最多）。**前提已落地（#897，2026-08-16）**：趋势块 Compose 现携同向延续中枢序列
/// （LevelExpansion 断链，M-1 盘整单中枢不动），真实数据（BTC 尾 300K）实测 41 个多中枢
/// Compose 全部判出方向、15 个 move 相对旧端点缝改判——三臂重测的区分力已具备。
///
/// 边界：中心少于两个时无法比较 M-2。盘整块（含 LevelExpansion 扩展链成员，M-1：盘整只含
/// 一个中枢）的 `Compose` 载荷恒为单中枢——此时**明确**退回既有首末子走势 `hi` 端点规则
/// （只作单中枢载荷兼容，不是第四套 M-2 判据；例外条款已按 #804 ③ 落
/// `.chanlun/definitions/qushi.md`，票号 #900）。趋势块（同向延续链）载荷 ≥2 中枢走 M-2，
/// **逐对相邻中枢全同向**才判该向（#900 F2——「依次同向」是相邻对，三枢先跌后涨时首尾
/// 比较会误判 Up）。子走势恰好一个时退回旧语义（`last.hi() >= first.hi()` 判 `Up`，#900 F3
/// 恢复——原 commit 只欲移除「空 `subs` 静默冒充 `Up`」）；空载荷仍 `None`。中心足够但
/// 所选判据既不向上也不向下时也返回 `None`；不会因判据失败而改用另一条判据兜底。#897 后
/// 趋势块已携真实中枢序列（前缀信息计算，全量/增量 bit-exact），#870 三臂重测可直接复用生产载荷。
pub fn rmove_dir(rmove: &RMove) -> Option<Direction> {
    rmove_dir_with_criterion(rmove, DEFAULT_DIR_CRITERION)
}

/// 按指定的 #815 M-2 候选臂判走势方向，供 #870 三臂重测。
pub fn rmove_dir_with_criterion(rmove: &RMove, criterion: DirCriterion) -> Option<Direction> {
    match rmove {
        RMove::Segment { direction, .. } => Some(*direction),
        RMove::Compose { subs, centers, .. } => {
            if centers.len() >= 2 {
                // #900 F2：逐对相邻判（「依次同向」），不再只比首尾。
                return criterion.classify_adjacent_pairs(centers);
            }
            legacy_sub_endpoint_dir(subs)
        }
    }
}

impl DirCriterion {
    fn classify(self, first: &Center, last: &Center) -> Option<Direction> {
        match self {
            Self::CoreSeparation => {
                classify_binary_relation(last.zd > first.zg, last.zg < first.zd)
            }
            Self::EnvelopeSeparation => {
                classify_binary_relation(last.dd > first.gg, last.gg < first.dd)
            }
            // ★#900 F1：第三臂（双升双降）保真复现 #815 底稿 Python 判据
            // `_centers_relation_by_zg_zd`——比的是中枢上下沿 **ZG/ZD**，不是外包络 GG/DD。
            // 旧实现比 GG/DD，合法输入即可判反（底稿反例：前枢 [0,4,10,20]、后枢 [−1,5,11,19]
            // ——按 ZG/ZD 判 Up，按 GG/DD 判 Down）。
            Self::DualEnvelopeRiseFall => classify_binary_relation(
                last.zg > first.zg && last.zd > first.zd,
                last.zg < first.zg && last.zd < first.zd,
            ),
        }
    }

    /// ★#900 F2：三中枢以上须「依次同向」——**逐对相邻**判（M-2 原文语义），全同向才取该向。
    /// 旧实现只比首尾，三枢先跌后涨时首尾仍可能判 Up。任一相邻对不确定或方向不一致 ⟹ `None`。
    fn classify_adjacent_pairs(self, centers: &[Center]) -> Option<Direction> {
        let mut acc: Option<Direction> = None;
        for pair in centers.windows(2) {
            let d = self.classify(&pair[0], &pair[1])?;
            match acc {
                None => acc = Some(d),
                Some(prev) if prev == d => {}
                Some(_) => return None,
            }
        }
        acc
    }
}

/// ★#883：父走势的最近中枢（D-3 取段的「界」）——Compose 取末中枢（盘整块 = 唯一中枢；
/// 趋势块 = 最后中枢 B，#897 后趋势块 Compose 携真实同向延续中枢序列）。Segment 无中枢
/// ⟹ None（div_cand 条件2 合法定位失败）。
pub fn parent_last_center(parent: &LeveledMove) -> Option<&Center> {
    match &parent.rmove {
        RMove::Compose { centers, .. } => centers.last(),
        RMove::Segment { .. } => None,
    }
}

fn classify_binary_relation(up: bool, down: bool) -> Option<Direction> {
    match (up, down) {
        (true, false) => Some(Direction::Up),
        (false, true) => Some(Direction::Down),
        _ => None,
    }
}

/// ★#900 F3：恢复单子走势旧语义——`subs.len() == 1` 时 `last.hi() >= first.hi()`（首末同一
/// 走势，恒真）判 `Up`。原 commit 只欲移除「空 `subs` 静默冒充 `Up`」，把单子走势一并
/// 打成 `None` 是票面意图外的改义（#900 追溯评审查实）。空载荷仍 `None`。
fn legacy_sub_endpoint_dir(subs: &[RMove]) -> Option<Direction> {
    let first = subs.first()?;
    let last = subs.last()?;
    Some(if last.hi() >= first.hi() {
        Direction::Up
    } else {
        Direction::Down
    })
}

/// #1028 裁定 A：一类点点锚 = departure 单元终点——`m.sub_moves` 中**最后一个**趋势方向
/// （`trend`）子走势的 `end_index`（趋势真终点）。
///
/// departure 单元 = 离开中枢的走势单元（一类点 C 段）。其 `end_index`（`compose`
/// `end_index = subs.last().end_index`，`recursive_tower.rs`）落在**末子段终点**——末子段常是
/// 回抽反趋势段（#1035 §1.4 实测 L1+ 首步反趋势占比 49%），点锚因此系统性晚于趋势真终点。
/// 本函数取回「最后一个 `rmove_dir == trend` 的子走势终点」作点锚（#1028 裁定 A，选项 B 极值
/// 锚已否决——极值不总落在末段终点，钉在段中间破坏「点=段终点」构造契约）。
///
/// 无趋势方向子走势（或 L0 空 `sub_moves`）⟹ `None`（诚实无键——调用方回退到走势自身
/// `end_index`，即旧锚口径）。
pub fn departure_unit_end(m: &LeveledMove, trend: Direction) -> Option<usize> {
    m.sub_moves
        .iter()
        .rev()
        .find_map(|sub| (rmove_dir(&sub.rmove) == Some(trend)).then_some(sub.end_index))
}

/// #814 D-3 统一取段（#1265）的最小投影——一个「可作力度比较基准」的走势单元。
///
/// 两个消费方（[`div_cand`] 条件2 的 [`LeveledMove`] 上下文、PanDiv 窄锚的 [`Segment`] 列表）
/// 形状不同，但都只消费五个量：方向（是否可判 + 是否同向）、起/终点坐标（进入段/离开段判定）、
/// 价格外缘 lo/hi（破核心判定）。本 trait 把它们投影到同一条 D-3 取段原语上，禁第二套实现
/// （#1265 收敛）。
pub(crate) trait D3Unit {
    fn direction(&self) -> Option<Direction>;
    fn start_index(&self) -> usize;
    fn end_index(&self) -> usize;
    fn lo(&self) -> Tick;
    fn hi(&self) -> Tick;
}

impl D3Unit for Segment {
    fn direction(&self) -> Option<Direction> {
        Some(self.direction)
    }
    fn start_index(&self) -> usize {
        self.start_index
    }
    fn end_index(&self) -> usize {
        self.end_index
    }
    fn lo(&self) -> Tick {
        self.start_price.min(self.end_price)
    }
    fn hi(&self) -> Tick {
        self.start_price.max(self.end_price)
    }
}

impl D3Unit for LeveledMove {
    fn direction(&self) -> Option<Direction> {
        rmove_dir(&self.rmove)
    }
    fn start_index(&self) -> usize {
        self.start_index
    }
    fn end_index(&self) -> usize {
        self.end_index
    }
    fn lo(&self) -> Tick {
        self.rmove.lo()
    }
    fn hi(&self) -> Tick {
        self.rmove.hi()
    }
}

/// 破核心判定（D-3「跨界」的几何谓词）：Down ⟹ `lo < zd`；Up ⟹ `hi > zg`。
///
/// 与 PanDiv 窄锚的 `end_price < c.zd / > c.zg` 同口径——L0 [`Segment`] 的端价即其 lo/hi 外缘
/// （Down 段端价最低 ⟹ `lo == end_price`；Up 段端价最高 ⟹ `hi == end_price`）；[`LeveledMove`]
/// 的 Compose 取外缘包络（[`RMove::lo`]/[`RMove::hi`]）。
pub(crate) fn d3_crosses_core<T: D3Unit>(m: &T, dir: Direction, c: &Center) -> bool {
    match dir {
        Direction::Down => m.lo() < c.zd,
        Direction::Up => m.hi() > c.zg,
    }
}

/// #814 D-3 统一取段：往回取最近同向跨界段（#1265 统一实现，全仓唯一取段原语）。
///
/// s' = `moves` 中时序最近的、方向 == `dir`、终点落在 `bound` 之前（`end_index <= bound`）、
/// 且为**跨界段**的单元。跨界段两类：进入段（`end_index <= c.start_index`）与离开段（破核心
/// Down ⟹ `lo < zd` / Up ⟹ `hi > zg`；`departure_only` 时另须 `start_index >= c.end_index`）。
/// 首次离开 = 进入段；反复震荡 = 上次离开段（`bound` = 当前离开 episode 起点 λ_C 时，「上次
/// 离开段」= 上一 episode 的同向离开段，与本次离开同 episode 的段被 `end <= bound` 排除）。
/// 中枢内震荡段（未破核心、非进入段）两谓词均不命中 ⟹ 跳过。无命中 ⟹ `None`（合法定位失败，非 bug）。
///
/// ★`bound` / `departure_only`（#1265）：两消费方的语境因递归层不同——[`div_cand`] 的
/// [`LeveledMove`] 语境下，单中心 Compose 的首次离开锚 = 首个中枢构成段（其 lo/hi 可破核心），
/// 取 `bound = s.start_index`、`departure_only = false`（破核心即跨界）；PanDiv 窄锚的
/// [`Segment`] 语境下，中枢材料段是构造材料、不参与力度比较（D-3 明文），且「上次离开段」须
/// 落在当前 episode 之前，取 `bound = λ_C`、`departure_only = true`（离开段须在中枢后
/// `start >= c.end`）。
pub(crate) fn d3_prev_crossing_anchor<'a, T: D3Unit>(
    moves: &'a [T],
    bound: usize,
    dir: Direction,
    c: &Center,
    departure_only: bool,
) -> Option<&'a T> {
    moves.iter().rfind(|m| {
        m.direction() == Some(dir)
            && m.end_index() <= bound
            && (((!departure_only || m.start_index() >= c.end_index)
                && d3_crosses_core(*m, dir, c))
                || m.end_index() <= c.start_index)
    })
}

/// DivCand^δ_{Θ,ℓ}(s,t)：背驰段候选四条件合取谓词。
///
/// ## 四条件（★#883 S4-b 后口径）
/// 1. **方向**：`dir(s) = −δ`（δ=Long→s 方向 Down；δ=Short→s 方向 Up）
/// 2. **Comparable（#814 D-3 统一取段，#1265 统一原语）**：比较基准 s' = 往回最近的**同方向
///    跨界段**——首次离开时 = 进入段（`end_index ≤ c.start_index` 的最近同向段）；反复震荡时 =
///    上一次冲出去的那段（同向且破核心：Down ⟹ `lo < c.zd`；Up ⟹ `hi > c.zg`）。中枢内部
///    震荡段不参与力度比较（D-3 明文）。取段经全仓唯一原语 [`d3_prev_crossing_anchor`]
///    （#1265：div_cand 条件2 与 PanDiv 窄锚收成一条）。前提：父走势最近中枢 `c` 存在且
///    **s 自身跨界**（「这次冲出中枢的那一段」；`c` = 父 Compose 末中枢，趋势块 = 最后中枢
///    B，盘整块 = 唯一中枢——#979 裁定一：趋势 c vs b 与盘整 C vs A 是同一条规则，b 即 B 的
///    进入段）。★不并存旧「最近同向段」无中枢口径（收敛通则：同一判断不得宽严两档）。
/// 3. **Extreme**：δ=Long→`lo(s)<lo(s')`；δ=Short→`hi(s)>hi(s')`（#814 D-2：次级别一类点
///    保留 Extreme；044:234 创新高/新低同为盘整背驰前提）。
/// 4. **Weak（#990 收编后统一判据原语，禁第二套力度引擎）**：
///    `confirm_divergence_l(gauge, macd_c_lt_a, None, l_c_lt_b)`——默认 `ForceL` 教义判据
///    `L(C)<L(B)`（#873，L 经 #989 `segment_force_l` 段→笔反查；无笔 ⟹ 无源不判，不降级）；
///    `MacdArea` 为显式对照档（同色柱面积，`is_divergence`）。
///
/// 任一条件不满足 ⟹ false（`Cand=0`，合法定位失败，非 bug）。
pub fn div_cand(input: &DivCandInput<'_>) -> bool {
    div_cand_fail(input).is_none()
}

/// ★#883：[`div_cand`] 的**失败分支外化**——返回首个不满足的条件号，`None` = 四条件全过。
///
/// 条件号：0=`target_idx` 越界；1=方向 `dir(s)≠−δ`；2=D-3 取段结构失败（无父中枢语境 /
/// s 未跨界 / 前序无同向跨界段）；3=Extreme 不成立；4=Weak 不成立。
///
/// 探针（#846 系 `type1_descend_continuity_dx` 等）直接消费本函数做条件归因——
/// 与生产判据**同一函数**，parity 由构造保证，不再需要镜像体 + 对拍锁。
pub fn div_cand_fail(input: &DivCandInput<'_>) -> Option<u8> {
    let DivCandInput {
        context,
        target_idx,
        hist,
        delta,
        strokes,
        parent_center,
        gauge,
        close_src,
    } = input;
    let context = *context;
    let target_idx = *target_idx;
    let hist = *hist;
    let delta = *delta;

    // 越界保护。
    if target_idx >= context.len() {
        return Some(0);
    }
    let s = &context[target_idx];
    // 方向/lo/hi 从 LeveledMove 惰性派生（== ContextMove 旧投影：dir=rmove_dir，lo/hi=rmove.lo()/hi()）。
    let Some(s_dir) = rmove_dir(&s.rmove) else {
        return Some(1);
    };

    // 条件1：dir(s) = −δ。
    let expected_dir = match delta {
        Side::Long => Direction::Down, // δ=买 → 背驰段方向 = 下跌
        Side::Short => Direction::Up,  // δ=卖 → 背驰段方向 = 上涨
    };
    if s_dir != expected_dir {
        return Some(1);
    }

    // 条件2（★#883 S4-b：#814 D-3 统一取段规则——趋势/盘整同一条，#979 裁定一；★#1265 取段
    // 原语收敛为全仓唯一 [`d3_prev_crossing_anchor`]，与 PanDiv 窄锚同函数——#1231 裁定 a）。
    // 「界」= 父走势最近中枢；无中枢语境 ⟹ 跨界无定义 ⟹ 合法定位失败（不退回旧无中枢口径）。
    let Some(c) = parent_center else {
        return Some(2);
    };
    // C（=s）自身须是「这次冲出中枢的那一段」（D-3 明文）；未跨界 = 中枢内震荡，非背驰段。
    if !d3_crosses_core(s, s_dir, c) {
        return Some(2);
    }
    // s' = 往回最近的同方向跨界段（首次离开 = 进入段 `end_index ≤ c.start_index`；反复震荡 =
    // 上一次冲出去的同向破核心段）；中枢内震荡段两谓词均不命中 ⟹ 跳过。
    let prev = d3_prev_crossing_anchor(context, s.start_index, s_dir, c, false);
    let Some(s_prev) = prev else { return Some(2) };

    // 条件3：Extreme。
    let extreme_ok = match delta {
        Side::Long => s.rmove.lo() < s_prev.rmove.lo(), // 下跌段更低低点
        Side::Short => s.rmove.hi() > s_prev.rmove.hi(), // 上涨段更高高点
    };
    if !extreme_ok {
        return Some(3);
    }

    // 条件4（★#883：Weak 走 #990 收编后统一判据原语——默认 ForceL 教义档，MacdArea 对照档，
    // 禁第二套力度引擎）。#988 同色口径：段方向 = delta 侧的反向走势（Long=下跌段 ⟹ Down
    // 取绿柱，segments_diverge_or 同款映射）。
    let dir = match delta {
        Side::Long => Direction::Down,
        Side::Short => Direction::Up,
    };
    // ★#1228：条件4 的 MACD 面积按 hist 坐标系取段——`close_src=Some` 时 hist 是 merged-bar 下标域
    // （pipeline 增量路径），须把段的 source_index 区间映射到 hist 下标；`None` 时 hist 即
    // source_index 域（econ_positive 全量路径），直读。映射失败（区间无 bar 落入）⟹ 面积 0.0。
    let (prev_area, curr_area) = match close_src {
        Some(src) => {
            let prev = map_src_to_close_idx(src, s_prev.start_index, s_prev.end_index);
            let curr = map_src_to_close_idx(src, s.start_index, s.end_index);
            (
                prev.map_or(0.0, |(a, b)| segment_macd_area(hist, a, b, dir)),
                curr.map_or(0.0, |(a, b)| segment_macd_area(hist, a, b, dir)),
            )
        }
        None => (
            segment_macd_area(hist, s_prev.start_index, s_prev.end_index, dir),
            segment_macd_area(hist, s.start_index, s.end_index, dir),
        ),
    };
    let macd_c_lt_a = is_divergence(prev_area, curr_area);
    // ForceL 教义判据 L(C)<L(B)（#873；段→笔反查 #989）：任一段区间无笔 ⟹ None ⟹ 无源不判。
    let l_c_lt_b = match (
        segment_force_l(strokes, s_prev.start_index, s_prev.end_index),
        segment_force_l(strokes, s.start_index, s.end_index),
    ) {
        (Some(lb), Some(lc)) => Some(lc < lb),
        _ => None,
    };
    // 无 5-proxy 源（ThetaDom/ThetaLex/Conjunction 档恒不判，见 DivCandInput.gauge 文档）。
    if confirm_divergence_l(*gauge, macd_c_lt_a, None, l_c_lt_b) {
        None
    } else {
        Some(4)
    }
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
    strokes: &[Stroke],
    gauge: DivergenceGauge,
) -> bool {
    // 1. 找候选段 s（end_index == source_index）。
    let level_moves = tower.get(level).map(|rc| rc.as_slice()).unwrap_or(&[]);
    let Some(target_pos) = find_move_by_end_index(level_moves, source_index) else {
        return false;
    };
    let s = &level_moves[target_pos];

    // 2. 在上级（tower[level+1]）找包含 s 的 Compose，取其 sub_moves 作 context。
    let upper_moves = tower.get(level + 1).map(|rc| rc.as_slice()).unwrap_or(&[]);
    let Some(parent) = upper_moves
        .iter()
        .find(|p| p.start_index <= s.start_index && p.end_index >= s.end_index)
    else {
        return false; // 无上级语境
    };
    let context_subs = &parent.sub_moves;
    if context_subs.is_empty() {
        return false;
    }

    // 3. 在 sub_moves 上直接二分找 target_idx（div_cand 直读 LeveledMove，不再投影 ContextMove）。
    let Some(target_idx) = find_move_by_end_index(context_subs.as_slice(), source_index) else {
        return false;
    };

    // ★#883：D-3 取段的「界」= 父 Compose 的最近中枢（盘整块单中枢 / 趋势块末中枢 B，#897 后
    // 趋势块携真实中枢序列）；父非 Compose 或无中枢 ⟹ None ⟹ div_cand 条件2 合法定位失败。
    let parent_center = parent_last_center(parent);

    div_cand(&DivCandInput {
        context: context_subs.as_slice(),
        target_idx,
        hist,
        delta,
        strokes,
        parent_center,
        gauge,
        close_src: None,
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::types::Center;
    use super::super::recursive_tower::{ElementId, LeveledMove};
    use super::*;

    // ── 测试工具 ──────────────────────────────────────────────────────────────

    /// div_cand 现直读 LeveledMove——测试段用 RMove::Segment 承载 lo/hi/direction（L0，sub_moves 空）。
    fn seg(direction: Direction, lo: i64, hi: i64, start: usize, end: usize) -> LeveledMove {
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

    fn down_seg(lo: i64, hi: i64, start: usize, end: usize) -> LeveledMove {
        seg(Direction::Down, lo, hi, start, end)
    }

    fn up_seg(lo: i64, hi: i64, start: usize, end: usize) -> LeveledMove {
        seg(Direction::Up, lo, hi, start, end)
    }

    /// hist 序列：每 bar 固定值，面积 = 值 × bar 数。
    fn flat_hist(val: f64, n: usize) -> Vec<f64> {
        vec![val; n]
    }

    fn center(dd: i64, zd: i64, zg: i64, gg: i64) -> Center {
        Center {
            dd,
            zd,
            zg,
            gg,
            start_index: 0,
            end_index: 0,
        }
    }

    /// ★#883：D-3 取段测试中枢——`start` 控制「进入段」边界（`end_index ≤ start` 的同向段 =
    /// 进入段）；`zd`/`zg` 控制「破核心」边界。外缘 dd/gg 取核心值（本组测试不消费外缘）。
    fn pan_center(zd: i64, zg: i64, start: usize) -> Center {
        Center {
            dd: zd,
            zd,
            zg,
            gg: zg,
            start_index: start,
            end_index: start,
        }
    }

    fn compose_rmove(subs: Vec<RMove>, centers: Vec<Center>) -> RMove {
        RMove::Compose {
            subs: Rc::new(subs),
            centers,
            level: 1,
        }
    }

    #[test]
    fn dir_criterion_default_tracks_production_constant() {
        assert_eq!(DirCriterion::default(), DEFAULT_DIR_CRITERION);
    }

    #[test]
    fn rmove_dir_defaults_to_center_envelope_separation() {
        let rmove = compose_rmove(
            vec![
                RMove::Segment {
                    direction: Direction::Up,
                    lo: 100,
                    hi: 200,
                },
                RMove::Segment {
                    direction: Direction::Down,
                    lo: 50,
                    hi: 100,
                },
            ],
            vec![center(0, 2, 4, 6), center(10, 12, 14, 16)],
        );

        assert_eq!(rmove_dir(&rmove), Some(Direction::Up));
    }

    #[test]
    fn rmove_dir_can_select_core_separation() {
        let rmove = compose_rmove(vec![], vec![center(0, 2, 4, 10), center(8, 11, 13, 16)]);

        assert_eq!(
            rmove_dir_with_criterion(&rmove, DirCriterion::CoreSeparation),
            Some(Direction::Up)
        );
    }

    #[test]
    fn rmove_dir_can_select_dual_envelope_rise_fall() {
        let rmove = compose_rmove(vec![], vec![center(0, 4, 10, 14), center(2, 6, 12, 16)]);

        assert_eq!(
            rmove_dir_with_criterion(&rmove, DirCriterion::DualEnvelopeRiseFall),
            Some(Direction::Up)
        );
    }

    #[test]
    fn rmove_dir_dual_rise_fall_uses_zg_zd_center_edges() {
        // ★#900 F1 底稿反例：前枢 [0,4,10,20]、后枢 [−1,5,11,19]。
        // 按 ZG/ZD（中枢上下沿，底稿 Python 判据）判 Up（11>10 且 5>4）；
        // 旧实现按 GG/DD 会判 Down（19<20 且 −1<0）——合法输入判反。
        let rmove = compose_rmove(vec![], vec![center(0, 4, 10, 20), center(-1, 5, 11, 19)]);

        assert_eq!(
            rmove_dir_with_criterion(&rmove, DirCriterion::DualEnvelopeRiseFall),
            Some(Direction::Up)
        );
    }

    #[test]
    fn rmove_dir_dual_rise_fall_zg_up_zd_down_is_undetermined() {
        // ZG 升但 ZD 降（外包络 GG/DD 双升）——双升双降必须两维同向，混向判 None。
        // 旧实现按 GG/DD 会判 Up（#900 F1 修掉的口径）。
        let rmove = compose_rmove(vec![], vec![center(0, 4, 10, 20), center(1, 3, 11, 21)]);

        assert_eq!(
            rmove_dir_with_criterion(&rmove, DirCriterion::DualEnvelopeRiseFall),
            None
        );
    }

    #[test]
    fn rmove_dir_all_criteria_treat_equality_boundaries_as_undetermined() {
        let cases = [
            // 核心分离：分别卡住上行 `last.zd > first.zg` 与下行 `last.zg < first.zd`。
            (
                DirCriterion::CoreSeparation,
                center(0, 2, 4, 6),
                center(1, 4, 5, 7),
                "core-up-equality",
            ),
            (
                DirCriterion::CoreSeparation,
                center(1, 4, 5, 7),
                center(0, 2, 4, 6),
                "core-down-equality",
            ),
            // 外缘分离：分别卡住上行 `last.dd > first.gg` 与下行 `last.gg < first.dd`。
            (
                DirCriterion::EnvelopeSeparation,
                center(0, 2, 4, 6),
                center(6, 7, 8, 9),
                "envelope-up-equality",
            ),
            (
                DirCriterion::EnvelopeSeparation,
                center(6, 7, 8, 9),
                center(0, 2, 4, 6),
                "envelope-down-equality",
            ),
            // 双升双降（#900 F1 后比 ZG/ZD）：zg/zd 任一维相等都不得放宽成同向。
            (
                DirCriterion::DualEnvelopeRiseFall,
                center(0, 2, 4, 10),
                center(1, 3, 4, 11),
                "dual-up-zg-equality",
            ),
            (
                DirCriterion::DualEnvelopeRiseFall,
                center(0, 2, 4, 10),
                center(1, 2, 5, 11),
                "dual-up-zd-equality",
            ),
            (
                DirCriterion::DualEnvelopeRiseFall,
                center(1, 3, 5, 10),
                center(0, 2, 5, 11),
                "dual-down-zg-equality",
            ),
            (
                DirCriterion::DualEnvelopeRiseFall,
                center(1, 3, 5, 10),
                center(0, 3, 4, 9),
                "dual-down-zd-equality",
            ),
        ];

        for (criterion, first, last, boundary) in cases {
            let rmove = compose_rmove(vec![], vec![first, last]);
            assert_eq!(
                rmove_dir_with_criterion(&rmove, criterion),
                None,
                "boundary={boundary}"
            );
        }
    }

    #[test]
    fn rmove_dir_criteria_are_directionally_symmetric() {
        let cases = [
            (
                DirCriterion::CoreSeparation,
                vec![center(8, 11, 13, 16), center(0, 2, 4, 10)],
            ),
            (
                DirCriterion::EnvelopeSeparation,
                vec![center(10, 12, 14, 16), center(0, 2, 4, 6)],
            ),
            (
                DirCriterion::DualEnvelopeRiseFall,
                vec![center(2, 6, 12, 16), center(0, 4, 10, 14)],
            ),
        ];

        for (criterion, centers) in cases {
            let rmove = compose_rmove(vec![], centers);
            assert_eq!(
                rmove_dir_with_criterion(&rmove, criterion),
                Some(Direction::Down),
                "criterion={criterion:?}"
            );
        }
    }

    #[test]
    fn rmove_dir_sparse_centers_use_explicit_legacy_endpoint_fallback() {
        let subs = vec![
            RMove::Segment {
                direction: Direction::Up,
                lo: 100,
                hi: 200,
            },
            RMove::Segment {
                direction: Direction::Down,
                lo: 50,
                hi: 100,
            },
        ];
        let no_center = compose_rmove(subs.clone(), vec![]);
        let one_center = compose_rmove(subs, vec![center(50, 60, 90, 200)]);

        for criterion in [
            DirCriterion::CoreSeparation,
            DirCriterion::EnvelopeSeparation,
            DirCriterion::DualEnvelopeRiseFall,
        ] {
            for rmove in [&no_center, &one_center] {
                assert_eq!(
                    rmove_dir_with_criterion(rmove, criterion),
                    Some(Direction::Down),
                    "零/单中心兼容缝不应冒充 criterion={criterion:?} 的 M-2 判定"
                );
            }
        }
    }

    #[test]
    fn rmove_dir_real_single_center_compose_matches_legacy_endpoint_direction() {
        let subs = vec![
            seg(Direction::Up, 100, 200, 0, 4),
            seg(Direction::Down, 80, 180, 5, 9),
            seg(Direction::Up, 90, 150, 10, 14),
        ];
        let composed = LeveledMove::compose(
            &subs,
            &[center(80, 100, 150, 200)],
            1,
            ElementId {
                level: 1,
                ordinal: 0,
            },
        );
        let RMove::Compose { centers, .. } = &composed.rmove else {
            panic!("LeveledMove::compose 必须产出 RMove::Compose");
        };
        assert_eq!(
            centers.len(),
            1,
            "单中心 run（盘整/扩展链成员）载荷契约；趋势块 ≥2 由 #897 锁另测"
        );

        // 旧算法的手算真值：末子走势 hi=150 < 首子走势 hi=200，故为 Down。
        for criterion in [
            DirCriterion::CoreSeparation,
            DirCriterion::EnvelopeSeparation,
            DirCriterion::DualEnvelopeRiseFall,
        ] {
            assert_eq!(
                rmove_dir_with_criterion(&composed.rmove, criterion),
                Some(Direction::Down),
                "单中心现役构造必须走 legacy fallback；criterion={criterion:?}"
            );
        }
    }

    #[test]
    fn rmove_dir_empty_subs_fallback_is_undetermined() {
        // ★#900 F3：只移除「空 subs 静默冒充 Up」——空载荷仍 None。
        let empty = compose_rmove(vec![], vec![]);

        assert_eq!(rmove_dir(&empty), None);
    }

    #[test]
    fn rmove_dir_single_sub_restores_legacy_up_semantics() {
        // ★#900 F3：单子走势恢复旧语义——last.hi() >= first.hi()（首末同一走势，恒真）判 Up。
        // 原 commit 把单子走势一并打成 None 是票面意图外的改义。
        let one_sub = compose_rmove(
            vec![RMove::Segment {
                direction: Direction::Down,
                lo: 10,
                hi: 20,
            }],
            vec![center(10, 12, 18, 20)],
        );

        assert_eq!(rmove_dir(&one_sub), Some(Direction::Up));
    }

    #[test]
    fn rmove_dir_three_centers_requires_adjacent_pairs_all_same_direction() {
        // ★#900 F2：三枢先涨后跌——首尾比较会判 Up（last.dd=7 > first.gg=6），
        // 但「依次同向」要求相邻对全同向：c1→c2 升、c2→c3 判不出 ⟹ 整体 None。
        let mixed = compose_rmove(
            vec![],
            vec![
                center(0, 2, 4, 6),
                center(10, 12, 14, 16),
                center(7, 8, 10, 20),
            ],
        );
        assert_eq!(rmove_dir(&mixed), None);

        // 票面点名形状（#900 验收「三枢先跌后涨判 None」）：c1→c2 跌、c2→c3 涨——
        // 旧实现只比首尾会判 Up（last.dd=7 > first.gg=6），逐对判因方向不一致 ⟹ None。
        let dip_then_rise = compose_rmove(
            vec![],
            vec![
                center(0, 2, 4, 6),
                center(-10, -8, -6, -4),
                center(7, 8, 10, 20),
            ],
        );
        assert_eq!(rmove_dir(&dip_then_rise), None);

        // 同款三枢全同向（单调升）⟹ Up。
        let monotone = compose_rmove(
            vec![],
            vec![
                center(0, 2, 4, 6),
                center(10, 12, 14, 16),
                center(20, 22, 24, 26),
            ],
        );
        assert_eq!(rmove_dir(&monotone), Some(Direction::Up));
    }

    #[test]
    fn rmove_dir_does_not_fallback_after_selected_criterion_fails() {
        let rmove = compose_rmove(
            vec![
                RMove::Segment {
                    direction: Direction::Down,
                    lo: 0,
                    hi: 10,
                },
                RMove::Segment {
                    direction: Direction::Up,
                    lo: 10,
                    hi: 20,
                },
            ],
            vec![center(0, 4, 10, 14), center(2, 6, 12, 16)],
        );

        assert_eq!(rmove_dir(&rmove), None);
        assert_eq!(
            rmove_dir_with_criterion(&rmove, DirCriterion::DualEnvelopeRiseFall),
            Some(Direction::Up)
        );
    }

    #[test]
    fn rmove_dir_segment_keeps_embedded_direction() {
        let segment = RMove::Segment {
            direction: Direction::Down,
            lo: 10,
            hi: 20,
        };

        assert_eq!(rmove_dir(&segment), Some(Direction::Down));
    }

    /// #1028 裁定 A 回归锁：`departure_unit_end` 取最后一个趋势方向子走势终点——
    /// 末子段是回抽反趋势段时点锚不落到末子段终点（趋势真终点语义）。
    #[test]
    fn departure_unit_end_picks_last_trend_direction_submove() {
        // 子走势 = L0 段（RMove::Segment，rmove_dir 直读段方向）。
        let d5 = seg(Direction::Down, 90, 110, 0, 5);
        let u8 = seg(Direction::Up, 80, 90, 5, 8);
        let d11 = seg(Direction::Down, 60, 80, 8, 11);
        let u13 = seg(Direction::Up, 50, 60, 11, 13);
        let parent = LeveledMove::compose(
            &[d5.clone(), u8.clone(), d11.clone()],
            &[center(60, 80, 100, 110)],
            1,
            ElementId {
                level: 1,
                ordinal: 0,
            },
        );
        // 末子段是 Down（同趋势）⟹ 取末子段终点。
        assert_eq!(departure_unit_end(&parent, Direction::Down), Some(11));

        // 末子段换成 Up（回抽反趋势）⟹ 点锚回退到最后一个 Down 子段终点 11，而非末段 13。
        let with_pullback = LeveledMove::compose(
            &[d5.clone(), u8.clone(), d11.clone(), u13.clone()],
            &[center(50, 60, 80, 110)],
            1,
            ElementId {
                level: 1,
                ordinal: 0,
            },
        );
        assert_eq!(with_pullback.end_index, 13, "末子段终点 = 回抽段终点");
        assert_eq!(
            departure_unit_end(&with_pullback, Direction::Down),
            Some(11),
            "趋势真终点 = 最后一个 Down 子段终点，非回抽段终点 13"
        );

        // 全反趋势（trend=Down，subs 全 Up）⟹ None（诚实无键）。
        let all_counter = LeveledMove::compose(
            &[u8, u13],
            &[center(50, 60, 80, 90)],
            1,
            ElementId {
                level: 1,
                ordinal: 0,
            },
        );
        assert_eq!(departure_unit_end(&all_counter, Direction::Down), None);

        // L0 线段（空 subs）⟹ None（调用方回退 seg.end_index）。
        assert_eq!(departure_unit_end(&d11, Direction::Down), None);
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
        let input = DivCandInput {
            context: &context,
            target_idx: 1,
            hist: &hist,
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(55, 110, 0)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
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
        let input = DivCandInput {
            context: &context,
            target_idx: 1,
            hist: &hist,
            delta: Side::Short,
            strokes: &[],
            parent_center: Some(&pan_center(45, 95, 0)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        assert!(!div_cand(&input), "方向不反（Short × Down）⟹ Cand=0");
    }

    // ── 条件2：Comparable（存在前序同向段） ───────────────────────────────────

    /// 无前序同向段（target=首段）⟹ Cand=0。
    #[test]
    fn cond2_no_previous_same_dir_returns_false() {
        // target_idx=0：前序为空，无 s'。
        let context = vec![down_seg(50, 100, 0, 4)];
        let hist = flat_hist(1.0, 5);
        let input = DivCandInput {
            context: &context,
            target_idx: 0,
            hist: &hist,
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(60, 110, 0)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        assert!(!div_cand(&input), "无前序同向段 ⟹ Cand=0");
    }

    /// 前序全为反向段（无同向段）⟹ Cand=0。
    #[test]
    fn cond2_only_opposite_dir_prev_returns_false() {
        let context = vec![
            up_seg(50, 100, 0, 4),    // 方向 Up，与 δ=Long 的候选段 Down 不同向
            up_seg(60, 110, 5, 9),    // 同上，非同向
            down_seg(30, 90, 10, 14), // s，δ=Long 方向 Down 正确
        ];
        let hist = flat_hist(1.0, 15);
        let input = DivCandInput {
            context: &context,
            target_idx: 2,
            hist: &hist,
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(40, 95, 0)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        assert!(!div_cand(&input), "前序无同向段 ⟹ 条件2 不满足");
    }

    // ── 条件3：Extreme（价格极值更进） ───────────────────────────────────────

    /// δ=Long：候选段 lo 不低于前段 lo ⟹ Cand=0。
    #[test]
    fn cond3_lo_not_lower_long_delta_returns_false() {
        let context = vec![
            up_seg(50, 100, 0, 4),    // 反向段（中间段，形成结构）
            down_seg(40, 90, 5, 9),   // s'，lo=40
            up_seg(45, 95, 10, 14),   // 反向
            down_seg(45, 90, 15, 19), // s，lo=45 >= lo(s')=40 ⟹ 不满足 Extreme
        ];
        // 前序同向跨界段 = context[1]（Down，lo=40；进入段口径）。
        // 设 area(s')=10，area(s)=8（力度满足），但 Extreme 不满足。
        let hist_adj: Vec<f64> = (0..20).map(|i| if i < 10 { 2.0 } else { 1.5 }).collect();
        let input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &hist_adj,
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
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
        let input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &hist,
            delta: Side::Short,
            strokes: &[],
            parent_center: Some(&pan_center(48, 90, 10)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
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
        // #988 同色口径：Long=下跌段 ⟹ Down ⟹ 绿柱（负 hist）。面积衰减语义同构。
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { -2.0 } else { -3.0 }).collect();
        let input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &hist,
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
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
        // #988 同色口径：负 hist（绿柱）。
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { -2.0 } else { -1.0 }).collect();
        let input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &hist,
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
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
        // #988 同色口径：Short=上涨段 ⟹ Up ⟹ 红柱（正 hist）。
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { 2.0 } else { 1.0 }).collect();
        let input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &hist,
            delta: Side::Short,
            strokes: &[],
            parent_center: Some(&pan_center(48, 95, 10)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        assert!(div_cand(&input), "四条件全满足 δ=Short ⟹ Cand=1");
    }

    // ── 坐标映射对拍（★#1228：close_src 双坐标系同输入同输出） ──────────────

    /// 对拍锁：#814 D-2 要求两路径同输入同输出。identity 映射（`close_src[k]==k`）必须与
    /// `close_src=None`（source_index 直读）逐位同结果——pipeline（merged-bar hist + close_src）
    /// 与 econ_positive（source_index hist + None）在无 inclusion 合并时是同一份输入。
    #[test]
    fn close_src_identity_mapping_parity_with_none() {
        // 上涨 → 下跌(s') → 上涨 → 下跌(s)：四条件全满足（MacdArea 档消费 hist 面积）。
        let context = vec![
            up_seg(50, 100, 0, 4),
            down_seg(40, 90, 5, 9),
            up_seg(45, 95, 10, 14),
            down_seg(30, 85, 15, 19),
        ];
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { -2.0 } else { -1.0 }).collect();
        let identity: Vec<usize> = (0..20).collect();
        let none_input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &hist,
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        let mapped_input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &hist,
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::MacdArea,
            close_src: Some(&identity),
        };
        assert!(div_cand(&none_input), "基准输入四条件全满足");
        assert_eq!(
            div_cand(&none_input),
            div_cand(&mapped_input),
            "identity close_src 映射必须与 None（source_index 直读）同输入同输出"
        );
    }

    /// 对拍锁（merged-bar 缺口坐标系）：inclusion 合并后 close_src 非连续（本测 [0,2,4,..,18]），
    /// MACD 段面积必须经 `map_src_to_close_idx` 把 source_index 区间映射到 merged 下标。手算：
    /// s'=[5,9]→merged[3,4]（area=1.0+1.0=2.0），s=[15,19]→merged[8,9]（area=1.5+1.5=3.0）⟹
    /// curr>prev ⟹ 力度未衰减 ⟹ Cand=0。若不映射（close_src=None）会把 s 区间越界读成 0.0、
    /// s' 区间误读 merged[5..=9]（area=6.0）⟹ 误判背驰 ⟹ 本测试锁住映射真实被消费。
    #[test]
    fn close_src_merged_gap_mapping_is_consumed() {
        let context = vec![
            up_seg(50, 100, 0, 4),
            down_seg(40, 90, 5, 9),
            up_seg(45, 95, 10, 14),
            down_seg(30, 85, 15, 19),
        ];
        // merged-bar 下标域 hist（len=10）：s' 映射区间 [3,4] 力度 1.0+1.0=2.0，
        // s 映射区间 [8,9] 力度 1.5+1.5=3.0（未衰减）。
        let merged_hist: Vec<f64> =
            vec![-1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.5, -1.5];
        let close_src: Vec<usize> = vec![0, 2, 4, 6, 8, 10, 12, 14, 16, 18];
        let mapped_input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &merged_hist,
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::MacdArea,
            close_src: Some(&close_src),
        };
        let naive_input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &merged_hist,
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        assert!(
            !div_cand(&mapped_input),
            "正确映射：curr=3.0 > prev=2.0 ⟹ 力度未衰减 ⟹ Cand=0"
        );
        assert!(
            div_cand(&naive_input),
            "不映射（None）会把 s 区间越界读 0.0、s' 误读 merged[5..=9]=6.0 ⟹ 误判背驰——反证映射必需"
        );
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
            strokes: &[],
            parent_center: Some(&pan_center(50, 100, 0)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        assert!(!div_cand(&input), "空 context ⟹ false");
    }

    /// target_idx 越界 ⟹ false。
    #[test]
    fn out_of_bounds_target_returns_false() {
        let context = vec![down_seg(40, 90, 0, 4)];
        let input = DivCandInput {
            context: &context,
            target_idx: 5, // 越界
            hist: &flat_hist(1.0, 10),
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(50, 100, 0)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        assert!(!div_cand(&input), "越界 target_idx ⟹ false");
    }

    /// hist 为空 ⟹ area=0.0 ⟹ 条件4 0 < 0 = false。
    #[test]
    fn empty_hist_area_zero_divergence_fails() {
        let context = vec![
            up_seg(50, 100, 0, 4),
            down_seg(40, 90, 5, 9), // s'
            up_seg(45, 95, 10, 14),
            down_seg(30, 85, 15, 19), // s，Extreme ✓
        ];
        let input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &[], // 空 hist → area=0.0 → 0 < 0 = false
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        assert!(!div_cand(&input), "hist 空 ⟹ area=0 ⟹ 条件4 不满足");
    }

    /// Cand=0 ⟹ NestRung.cand=false ⟹ NestCertificate.n_delta()=false。
    /// 验证 Cand 向上游传播为 NestCertificate 的 0 值（规格 N^δ：任一级 Cand=0 ⟹ 整体 0）。
    #[test]
    fn cand_false_propagates_to_n_delta_zero() {
        use super::super::super::types::{BspBits, Side as CertSide};
        use super::super::nest::{NestCertificate, NestInterval, NestRung};

        // Cand=false 场景（前序无同向段）。
        let context = vec![down_seg(50, 100, 0, 4)];
        let hist = flat_hist(1.0, 5);
        let input = DivCandInput {
            context: &context,
            target_idx: 0,
            hist: &hist,
            delta: CertSide::Long,
            strokes: &[],
            parent_center: Some(&pan_center(60, 110, 0)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        let cand = div_cand(&input); // false
        assert!(!cand);

        // 喂入 NestCertificate：cand=false ⟹ n_delta()=false。
        let mut terminal = BspBits::default();
        terminal.buy1 = true; // 基例 Conf^+ = true
        let cert = NestCertificate::from_parts(
            CertSide::Long,
            terminal,
            NestInterval {
                end_index: 4,
                start_index: 0,
                idx: 0,
            },
            vec![NestRung::new(
                NestInterval {
                    end_index: 9,
                    start_index: 0,
                    idx: 0,
                },
                cand, // false
            )],
        );
        assert!(
            !cert.n_delta(),
            "任一级 Cand=false ⟹ n_delta()=false（N^δ 定义）"
        );
    }

    /// Cand=true ⟹ NestCertificate 其他条件满足时 n_delta()=true（多级链正例）。
    #[test]
    fn cand_true_with_valid_chain_n_delta_true() {
        use super::super::super::types::{BspBits, Side as CertSide};
        use super::super::nest::{NestCertificate, NestInterval, NestRung};

        // Cand=true（四条件全满足）。
        let context = vec![
            up_seg(50, 100, 0, 4),
            down_seg(40, 90, 5, 9), // s'
            up_seg(45, 95, 10, 14),
            down_seg(30, 85, 15, 19), // s
        ];
        // #988 同色口径：负 hist（绿柱）。
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { -2.0 } else { -1.0 }).collect();
        let input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &hist,
            delta: CertSide::Long,
            strokes: &[],
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        let cand = div_cand(&input);
        assert!(cand, "前置：四条件满足 Cand=true");

        // 构造两级证书（执行级 e=0，操作级 ℓ=1）。
        let mut terminal = BspBits::default();
        terminal.buy1 = true;
        // 执行级区间 [15,19]，操作级区间 [0,19]（⊇ 执行级）。
        let base = NestInterval {
            end_index: 19,
            start_index: 15,
            idx: 0,
        };
        let op_rung = NestRung::new(
            NestInterval {
                end_index: 19,
                start_index: 0,
                idx: 0,
            },
            cand,
        );
        let cert = NestCertificate::from_parts(CertSide::Long, terminal, base, vec![op_rung]);
        // is_sub(base, op_rung.interval)：[15,19] ⊆ [0,19] ✓。
        assert!(
            cert.n_delta(),
            "Cand=true + 区间套成立 + Conf^+ ⟹ n_delta()=true"
        );
    }

    // ── bsp_div_cand：从塔定位候选段并计算 DivCand ──────────────────────────

    use super::super::descend::RMove as TestRMove;

    /// 构建合成 LeveledMove（L0 Segment）。
    fn seg_move(
        dir: Direction,
        lo: i64,
        hi: i64,
        start: usize,
        end: usize,
        ord: u64,
    ) -> LeveledMove {
        use std::rc::Rc;
        let rmove = TestRMove::Segment {
            direction: dir,
            lo,
            hi,
        };
        LeveledMove {
            rmove,
            start_index: start,
            end_index: end,
            sub_moves: Rc::new(Vec::new()),
            id: ElementId {
                level: 0,
                ordinal: ord,
            },
        }
    }

    /// 构建合成 Compose LeveledMove（包含 sub_moves）。
    ///
    /// ★#883：中枢核心取 [45,92]、start_index=10——s3（lo=30 / hi 侧对称）破核心跨界、
    /// s1（end=9 ≤ 10）命中进入段口径，D-3 取段在本组夹具上落到 s1（与旧「最近同向段」同段，
    /// 两测试语义不变）。
    fn compose_move(subs: Vec<LeveledMove>, level: u32, ord: u64) -> LeveledMove {
        use std::rc::Rc;
        let start = subs.first().map(|m| m.start_index).unwrap_or(0);
        let end = subs.last().map(|m| m.end_index).unwrap_or(0);
        let sub_rmoves: Vec<_> = subs.iter().map(|m| m.rmove.clone()).collect();
        LeveledMove {
            rmove: TestRMove::Compose {
                subs: Rc::new(sub_rmoves),
                centers: vec![Center {
                    zd: 45,
                    zg: 92,
                    dd: lo_of(&subs),
                    gg: hi_of(&subs),
                    start_index: 10,
                    end_index: end,
                }],
                level,
            },
            start_index: start,
            end_index: end,
            sub_moves: Rc::new(subs),
            id: ElementId {
                level,
                ordinal: ord,
            },
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
        assert!(
            !super::bsp_div_cand(
                &tower,
                0,
                5,
                Side::Long,
                &hist,
                &[],
                DivergenceGauge::MacdArea
            ),
            "空塔 ⟹ false"
        );
    }

    /// bsp_div_cand：塔中有父 Compose，但 sub_moves 中无前序同向段 ⟹ false（条件2 不满足）。
    #[test]
    fn bsp_div_cand_no_prev_same_dir_returns_false() {
        use std::rc::Rc;

        // L0：4段（up/down/up/down），目标段 source_index=19（最后一段 Down end=19）
        let s0 = seg_move(Direction::Up, 50, 100, 0, 4, 0);
        let s1 = seg_move(Direction::Down, 40, 90, 5, 9, 1);
        let s2 = seg_move(Direction::Up, 45, 95, 10, 14, 2);
        let s3 = seg_move(Direction::Down, 30, 85, 15, 19, 3); // target

        // L1：Compose（包含全部4个 L0 段）
        let parent = compose_move(vec![s0.clone(), s1.clone(), s2.clone(), s3.clone()], 1, 0);

        let tower: Vec<Rc<Vec<_>>> = vec![
            Rc::new(vec![s0, s1, s2, s3]), // L0
            Rc::new(vec![parent]),         // L1
        ];
        // s3（Down）的前序同向段=s1（Down）→ 应有前序同向段；但 s3.lo=30 < s1.lo=40（Extreme ✓）。
        // 力度：若 hist 全 0 ⟹ area=0 ⟹ 0 < 0 = false（条件4 不满足）。
        let hist = flat_hist(0.0, 20);
        assert!(
            !super::bsp_div_cand(
                &tower,
                0,
                19,
                Side::Long,
                &hist,
                &[],
                DivergenceGauge::MacdArea
            ),
            "hist=0 ⟹ 条件4 不满足 ⟹ false"
        );
    }

    /// bsp_div_cand：四条件全满足 ⟹ true。
    #[test]
    fn bsp_div_cand_all_conditions_true() {
        use std::rc::Rc;

        // 同 all_four_conditions_satisfied_long_returns_true 的结构。
        let s0 = seg_move(Direction::Up, 50, 100, 0, 4, 0);
        let s1 = seg_move(Direction::Down, 40, 90, 5, 9, 1); // s'：Down，lo=40
        let s2 = seg_move(Direction::Up, 45, 95, 10, 14, 2);
        let s3 = seg_move(Direction::Down, 30, 85, 15, 19, 3); // s：Down，lo=30 < 40

        let parent = compose_move(vec![s0.clone(), s1.clone(), s2.clone(), s3.clone()], 1, 0);
        let tower: Vec<Rc<Vec<_>>> = vec![Rc::new(vec![s0, s1, s2, s3]), Rc::new(vec![parent])];
        // hist：s'(5-9) area=5*2=10，s(15-19) area=5*1=5 < 10（条件4 ✓）。
        // #988 同色口径：负 hist（绿柱）。
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { -2.0 } else { -1.0 }).collect();
        assert!(
            super::bsp_div_cand(
                &tower,
                0,
                19,
                Side::Long,
                &hist,
                &[],
                DivergenceGauge::MacdArea
            ),
            "四条件全满足 ⟹ bsp_div_cand = true"
        );
    }

    // ── ★#883 S4-b：盘整背驰入口（#814 D-3 统一取段）+ ForceL 统一力度原语（#990） ──

    /// 合成笔（ForceL 测试夹具；字段与 parser::stroke::Stroke 同源）。
    fn stroke(dir: Direction, start: usize, end: usize, sp: i64, ep: i64) -> Stroke {
        Stroke {
            direction: dir,
            start_index: start,
            end_index: end,
            start_price: sp,
            end_price: ep,
        }
    }

    /// 中枢内震荡段不参与力度比较（D-3 明文「中枢内部的震荡段是中枢的构造材料」）。
    ///
    /// 判别性夹具：震荡段（lo=60）若被误取为 s'，Extreme（48<60）成立 → 误判 true；
    /// 正确取段 s'=进入段（lo=45），Extreme（48<45）不成立 ⟹ cond3 false。
    #[test]
    fn cond2_intracenter_oscillation_segment_is_not_comparable() {
        let context = vec![
            down_seg(45, 90, 5, 9),   // 进入段（end 9 ≤ c.start=10）
            down_seg(60, 80, 10, 14), // 中枢内同向震荡段（lo=60 ≥ zd=50 未破核心、非进入段）
            down_seg(48, 70, 15, 19), // s：跨界（lo=48 < zd=50）
        ];
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { -2.0 } else { -1.0 }).collect();
        let input = DivCandInput {
            context: &context,
            target_idx: 2,
            hist: &hist,
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        assert_eq!(
            div_cand_fail(&input),
            Some(3),
            "震荡段不得作比较基准；s'=进入段（lo=45）时 Extreme 48<45 不成立"
        );
        assert!(!div_cand(&input));
    }

    /// 盘整背驰入口正例（首次离开）：D-3 取段 s'=进入段，四条件全过 ⟹ true。
    ///
    /// 这是 #883 前**没有入口**的类别：旧「同父前序最近同向段」口径下本夹具的 s' 也是
    /// context[0]，但中枢内若有更近的同向震荡段旧口径会错取——入口的形状差异见
    /// `cond2_intracenter_oscillation_segment_is_not_comparable`。
    #[test]
    fn pan_div_first_exit_compares_entry_segment_returns_true() {
        let context = vec![
            down_seg(40, 90, 5, 9),   // 进入段（D-3 首次离开的比较对象）
            up_seg(60, 92, 10, 14),   // 中枢内震荡（反向）
            down_seg(30, 85, 15, 19), // s：破核心（lo=30 < zd=50）
        ];
        // s'=[5,9] area=5×2=10；s=[15,19] area=5×1=5 < 10（MacdArea 对照档 Weak ✓）。
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { -2.0 } else { -1.0 }).collect();
        let input = DivCandInput {
            context: &context,
            target_idx: 2,
            hist: &hist,
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        assert_eq!(
            div_cand_fail(&input),
            None,
            "首次离开：s'=进入段，Extreme 30<40 ✓、Weak 5<10 ✓"
        );
        assert!(div_cand(&input));
    }

    /// 盘整背驰入口（反复震荡）：D-3 取段 s'=上一次冲出去的同向段，**不是**进入段。
    ///
    /// 判别性夹具：进入段 lo=38、上次离开段 lo=40、s lo=39——s' 取上次离开段时
    /// Extreme（39<40）成立；误取进入段则 39<38 不成立。
    #[test]
    fn pan_div_repeated_oscillation_compares_previous_exit() {
        let context = vec![
            down_seg(38, 90, 5, 9),   // 进入段（end 9 ≤ c.start=10）
            up_seg(60, 92, 10, 11),   // 震荡（反向）
            down_seg(40, 88, 12, 14), // 上一次离开段（lo=40 < zd=50 破核心）
            up_seg(60, 80, 15, 19),   // 中枢内震荡（反向）
            down_seg(39, 70, 20, 24), // s（lo=39 < 50 跨界）
        ];
        // s'=[12,14] area=3×2=6；s=[20,24] area=5×1=5 < 6（Weak ✓）。
        let hist: Vec<f64> = (0..25)
            .map(|i| {
                if (12..=14).contains(&i) {
                    -2.0
                } else if i >= 20 {
                    -1.0
                } else {
                    0.0
                }
            })
            .collect();
        let input = DivCandInput {
            context: &context,
            target_idx: 4,
            hist: &hist,
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        assert_eq!(
            div_cand_fail(&input),
            None,
            "反复震荡：s'=上一次离开段（时序最近同向跨界段），Extreme 39<40 ✓"
        );
        assert!(div_cand(&input));
    }

    /// C（=s）自身未冲出中枢核心 ⟹ 中枢内震荡，非背驰段 ⟹ cond2 false（D-3：比较对是
    /// 「这次冲出中枢的那一段」；未跨界即无盘整背驰可言）。
    #[test]
    fn cond2_target_not_crossing_center_returns_false() {
        let context = vec![
            down_seg(40, 90, 5, 9),   // 进入段
            up_seg(60, 92, 10, 14),   // 震荡
            down_seg(55, 70, 15, 19), // s：lo=55 ≥ zd=50 **未破核心**
        ];
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { -2.0 } else { -1.0 }).collect();
        let input = DivCandInput {
            context: &context,
            target_idx: 2,
            hist: &hist,
            delta: Side::Long,
            strokes: &[],
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        assert_eq!(
            div_cand_fail(&input),
            Some(2),
            "s 未跨界 ⟹ 条件2 取段结构失败（即便 Extreme/Weak 形式上可满足）"
        );
    }

    /// 无父中枢语境 ⟹ 条件2 合法定位失败（跨界无定义）。
    ///
    /// ★收敛通则锁：不得退回旧「同父前序最近同向段」无中枢口径——同一判断不并存两套
    /// 取段判据（#979 裁定一：D-3 是唯一取段规则）。
    #[test]
    fn cond2_no_parent_center_returns_false() {
        // 与 all_four_conditions_satisfied_long_returns_true 同一夹具，仅缺 parent_center。
        let context = vec![
            up_seg(50, 100, 0, 4),
            down_seg(40, 90, 5, 9),
            up_seg(45, 95, 10, 14),
            down_seg(30, 85, 15, 19),
        ];
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { -2.0 } else { -1.0 }).collect();
        let input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &hist,
            delta: Side::Long,
            strokes: &[],
            parent_center: None,
            gauge: DivergenceGauge::MacdArea,
            close_src: None,
        };
        assert_eq!(
            div_cand_fail(&input),
            Some(2),
            "无父中枢 ⟹ 条件2 失败（不退回旧口径）"
        );
    }

    /// ★#1265 对拍锁（同输入同输出）：D-3 取段原语对 [`LeveledMove`]（div_cand 语境）与
    /// [`Segment`]（PanDiv 窄锚语境）投影同一组走势时，返回同一锚（同方向/同区间/同外缘）。
    #[test]
    fn d3_prev_crossing_anchor_parity_leveled_move_vs_segment() {
        // 同一组走势（反复震荡）：进入段 → 反向震荡 → 上次离开段 → 反向震荡 → 目标离开段。
        let c = pan_center(50, 96, 10);
        let dir = Direction::Down;
        let leveled = vec![
            down_seg(40, 90, 5, 9),   // 进入段（end 9 <= c.start 10）
            up_seg(60, 92, 10, 14),   // 中枢内震荡（反向）
            down_seg(30, 88, 15, 19), // 上次离开段（lo 30 < zd 50）
            up_seg(60, 80, 20, 24),   // 中枢内震荡（反向）
            down_seg(25, 70, 25, 29), // 目标（lo 25 < zd 50）
        ];
        // types::Segment 投影：Down 段 start_price=hi、end_price=lo（端价即 lo/hi 外缘）。
        let raw = |d: Direction, lo: i64, hi: i64, start: usize, end: usize| Segment {
            direction: d,
            start_index: start,
            end_index: end,
            start_price: if d == Direction::Down { hi } else { lo },
            end_price: if d == Direction::Down { lo } else { hi },
        };
        let segs = vec![
            raw(Direction::Down, 40, 90, 5, 9),
            raw(Direction::Up, 60, 92, 10, 14),
            raw(Direction::Down, 30, 88, 15, 19),
            raw(Direction::Up, 60, 80, 20, 24),
            raw(Direction::Down, 25, 70, 25, 29),
        ];

        // 同一 bound（目标 start 25）与 departure_only=true 口径下，两投影返回同一锚（索引 2）。
        let lvl_anchor = d3_prev_crossing_anchor(&leveled, 25, dir, &c, true)
            .expect("LeveledMove 投影应命中上次离开段");
        let seg_anchor = d3_prev_crossing_anchor(&segs, 25, dir, &c, true)
            .expect("Segment 投影应命中上次离开段");
        assert_eq!(lvl_anchor.start_index(), 15, "锚 = 上次离开段");
        assert_eq!(
            seg_anchor.start_index(),
            lvl_anchor.start_index(),
            "同输入同输出（start）"
        );
        assert_eq!(
            seg_anchor.end_index(),
            lvl_anchor.end_index(),
            "同输入同输出（end）"
        );
        assert_eq!(
            seg_anchor.direction(),
            lvl_anchor.direction(),
            "同输入同输出（direction）"
        );
        assert_eq!(seg_anchor.lo(), lvl_anchor.lo(), "同输入同输出（lo）");
        assert_eq!(seg_anchor.hi(), lvl_anchor.hi(), "同输入同输出（hi）");

        // 首次离开（中枢后无同向离开段，仅进入段）：两投影同落进入段（索引 0）。
        let first_leveled = vec![down_seg(40, 90, 5, 9), down_seg(30, 85, 15, 19)];
        let first_segs = vec![
            raw(Direction::Down, 40, 90, 5, 9),
            raw(Direction::Down, 30, 85, 15, 19),
        ];
        let lvl_entry = d3_prev_crossing_anchor(&first_leveled, 15, dir, &c, true)
            .expect("首次离开：LeveledMove 投影应命中进入段");
        let seg_entry = d3_prev_crossing_anchor(&first_segs, 15, dir, &c, true)
            .expect("首次离开：Segment 投影应命中进入段");
        assert_eq!(lvl_entry.start_index(), 5, "锚 = 进入段");
        assert_eq!(
            seg_entry.start_index(),
            lvl_entry.start_index(),
            "同输入同输出（首次离开 start）"
        );
    }

    /// ★ForceL 教义档（生产默认，#990）：条件4 = `L(C)<L(B)`，L 经段→笔反查（#873/#989）。
    ///
    /// 手工验算（向下笔速度取沿走势方向速率，恒正）：s'=[5,9] 首笔 v=(90−60)/3=10、
    /// 末笔 v=(60−40)/3≈6.67 ⟹ L(b)≈−3.33；s=[15,19] 首笔 v=(85−40)/3=15、
    /// 末笔 v=(40−30)/3≈3.33 ⟹ L(c)≈−11.67 < L(b) ⟹ 判背驰。
    #[test]
    fn cond4_forcel_compares_stroke_velocity_increment() {
        let context = vec![
            up_seg(50, 100, 0, 4),
            down_seg(40, 90, 5, 9),
            up_seg(45, 95, 10, 14),
            down_seg(30, 85, 15, 19),
        ];
        let strokes = vec![
            stroke(Direction::Down, 5, 7, 90, 60),
            stroke(Direction::Down, 7, 9, 60, 40),
            stroke(Direction::Down, 15, 17, 85, 40),
            stroke(Direction::Down, 17, 19, 40, 30),
        ];
        let input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &[], // ForceL 档不消费 hist
            delta: Side::Long,
            strokes: &strokes,
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::ForceL,
            close_src: None,
        };
        assert_eq!(
            div_cand_fail(&input),
            None,
            "L(c)≈−11.67 < L(b)≈−3.33 ⟹ ForceL 档四条件全过"
        );
    }

    /// ForceL 档反例：L(c) ≥ L(b)（衰减更浅）⟹ 条件4 不成立。
    ///
    /// 手工验算：s' 首笔 v=(90−30)/3=20、末笔 v=(40−36)/3≈1.33 ⟹ L(b)≈−18.67；
    /// s 首笔 v=(85−50)/3≈11.67、末笔 v=(50−30)/3≈6.67 ⟹ L(c)=−5.0 ≥ L(b) ⟹ 不判。
    #[test]
    fn cond4_forcel_no_weakening_returns_false() {
        let context = vec![
            up_seg(50, 100, 0, 4),
            down_seg(40, 90, 5, 9),
            up_seg(45, 95, 10, 14),
            down_seg(30, 85, 15, 19),
        ];
        let strokes = vec![
            stroke(Direction::Down, 5, 7, 90, 30),
            stroke(Direction::Down, 7, 9, 40, 36),
            stroke(Direction::Down, 15, 17, 85, 50),
            stroke(Direction::Down, 17, 19, 50, 30),
        ];
        let input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &[],
            delta: Side::Long,
            strokes: &strokes,
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::ForceL,
            close_src: None,
        };
        assert_eq!(
            div_cand_fail(&input),
            Some(4),
            "L(c)=−5.0 ≥ L(b)≈−18.67 ⟹ ForceL 档条件4 不成立"
        );
    }

    /// ForceL 档无笔数据源 ⟹ 诚实不判（None → false），**不降级**到 MacdArea——
    /// 收敛通则禁宽松档接管严格档的失败（#990 同款「无源不判」）。
    #[test]
    fn cond4_forcel_without_strokes_honest_no_judgment() {
        let context = vec![
            up_seg(50, 100, 0, 4),
            down_seg(40, 90, 5, 9),
            up_seg(45, 95, 10, 14),
            down_seg(30, 85, 15, 19),
        ];
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { -2.0 } else { -1.0 }).collect();
        let input = DivCandInput {
            context: &context,
            target_idx: 3,
            hist: &hist, // MacdArea 对照读数满足（5<10），但 ForceL 档不消费
            delta: Side::Long,
            strokes: &[], // 无笔 ⟹ L 无源
            parent_center: Some(&pan_center(50, 96, 10)),
            gauge: DivergenceGauge::ForceL,
            close_src: None,
        };
        assert_eq!(
            div_cand_fail(&input),
            Some(4),
            "ForceL 无源 ⟹ 条件4 不判（不降级 MacdArea）"
        );
    }
}
