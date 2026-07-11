//! 递归塔对象（携坐标的 `RMove::Compose` 塔）——#53 still-MISSING-塔升级。
//!
//! ## 工位定位（递归塔 UnitRange→RMove::Compose 升级，task #53）
//!
//! `mod.rs::classify` 的递归塔旧版每级走势单元是 [`super::center::UnitRange`]（中枢外缘区间，
//! **无递归 subs**）——`extract_second_signals`（消费 `RMove::Compose` 的 `descend` 取回次级别
//! 走势序列）在此塔上**永产不出 B2/S2**（descend 得空）。本文件把递归塔的走势单元升级为
//! **携次级别走势 subs 的 [`RMove::Compose`]**（descend.rs port `Origin.SubLevelDescent.RMove`
//! = Lean `Move` μF），完成后 `extract_second_signals` **零改动**真接入生产路径。
//!
//! ## 三层对象的存在论分工（no-patch：结构层 / 坐标层 / 力度层各司其职）
//!
//! | 层 | 对象 | 携带 | 来源 |
//! |----|------|------|------|
//! | 结构层 | [`RMove`]（descend.rs）| direction/lo/hi（Segment）∨ subs/centers/level（Compose）| Lean `Move` μF bit-exact 镜像 |
//! | 坐标层 | [`LeveledMove`]（本文件）| `rmove` + `start_index`/`end_index`（原始 K 序）| 生产塔关注点（Lean μF 不含坐标）|
//! | 力度层 | MACD 背驰 bool | 次级别走势的 close 区间 → MACD 面积比较 | `divergence.rs`（rust 领先 Origin）|
//!
//! ★为什么坐标不塞进 `RMove`（no-声明膨胀）：`RMove`（descend.rs）是 Lean `Move` μF 的逐字段
//! 镜像（`Move.segment d lo hi` / `Move.compose subs centers level`，**无 source_index**——Lean
//! μF 是纯结构区间）。在 `RMove::Segment` 上硬加 `source_index` = 破坏与 Lean 的 bit-exact 对齐
//! （声明 Lean μF 不具备的字段）。严格的做法：坐标在**包装层** [`LeveledMove`] 携带，结构层
//! `RMove` 保持纯净。`extract_second_signals` 的 `index_of` 闭包从 [`LeveledMove`] 的坐标实现——
//! 坐标随结构一起传递（侧车同构），无事后查表歧义（两个区间相同的 RMove 不会混淆坐标）。
//!
//! ## 窗口化 compose 语义（契约锚 `Origin.RecursiveLevelSystem.composeStep` + 走势分解定理二）
//!
//! L(k+1) 的每个上级走势单元 = 构成一个中枢的那**连续三段**次级别 [`LeveledMove`] 经 [`compose`]
//! 封装（`RMove::Compose { subs: 三段次级别 rmove, centers: [该中枢], level: k+1 }`）。这与 Lean
//! `composeStep`（规范窗口族切多窗，每窗封装一个上级走势，中枢由窗口三段区间重叠真派生）逐字段
//! 同构——上级走势的 subs **正是** `descend` 取回的次级别走势序列（组装-取回对偶 `descend ∘ compose = id`）。
//! 旧塔把中枢折叠成无 subs 的 `UnitRange`（丢弃构成它的三段次级别走势）；新塔保留三段次级别走势
//! 作 subs，故 descend 能取回它们 ⟹ B2/S2 可产。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - **L0**（结构）：递归塔对象 [`LeveledMove`]（RMove::Compose + 坐标）的构造是纯结构操作
//!   （窗口 compose + 坐标传递），不依赖经验数据。`descend ∘ compose = id` 是 L0 对偶律。
//! - **L1**（端到端管线正确性）：mod.rs::classify 构造塔 + `extract_second_signals` 在塔上产 B2/S2
//!   = bit-exact 一致性验证（验证管线正确，**不**验证「第二类识别在真实行情有效」）。
//! - **still-MISSING**（诚实开口，见 mod.rs::classify 接入点 + 本文件 §still-MISSING）：背驰力度
//!   引擎的「次级别走势 → MACD 背驰」自动配对依赖次级别走势携带 close 区间——本塔的 `LeveledMove`
//!   携 source_index 坐标（可定位 close 区间），但「次级别 close 区间 → MACD 面积比较」的自动闭包
//!   接入是 L1 力度配对，由 mod.rs 接入点用 `divergence.rs` 真算（见接入点诚实标注）。

use std::rc::Rc;

use super::super::types::{Center, Direction, Tick};
use super::center::UnitRange;
use super::decompose::{center_own_dir_at, MoveBlock};
use super::descend::RMove;

/// ★#148 升级阈值（第33课，结构常数非参数——codex 裁定 A1，codex-decide-20260704-001933）：
/// 「中枢的延伸不能超过5段，也就是一旦出现6段的延伸，加上形成中枢本身那三段，就构成更大级别
/// 的中枢了」——窗口总段数（seed 3 + 延伸）达 9 ⟹ 不再是本级别延伸中枢，按每 3 段重切为本级别
/// 子中枢（「每3段构成一个中枢」），升级经上一级 detect 自然涌现。第20课「延伸不升级」（涨停例）
/// 与本约定的调和（裁定 D1）：33课是晚出的操作层多义性消解约定；涨停例=时间延伸而非完成段数
/// 增长，段数口径下自洽不触发。
pub const UPGRADE_TOTAL_SEGMENTS: usize = 9;

/// 确定性元素身份（codex Q4：spec §13 `p:C_ℓ→C_{ℓ+1}` 结构映射的 rust 对象身份）。
///
/// 跨 bar 稳定（全量/增量产同 ID），非 per-bar `Vec` 索引——解 codex 发现 A/B：
/// - 发现 A：`held_leg_tree_index` Stale 分支伪造 `parent:None`（AncOK 放宽，spec §13 line 671 偏差）。
/// - 发现 B：值比较 `(level,λ,eps)` 非 spec §13 结构映射；真因 = 每 bar 重建 Vec + 更高级根重构索引重映射。
///
/// `id = (level, ordinal)`：级别 + 该级 compose 输出序号（同级别窗口扫描产出序，全量/增量一致）。
/// L0 线段 ordinal = 段在 L0 序列中的索引；上级 Compose ordinal = 窗口在该级产出序（增量 tail 接续前缀）。
///
/// ★确定性保证（全量/增量产同 ID）：`compose_level` 用 `enumerate` 注入 ordinal；
/// `compose_level_resume` 接收 `prefix_count`（已产出前缀数），tail ordinal = `prefix_count + i`。
///
/// ## 认识论等级（formalization-validity-domain 231号）
/// **L0**（纯结构身份）：ID 构造是确定性序号注入，不依赖经验数据。跨 bar 稳定是定义性（同前缀同 ID）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElementId {
    /// 级别 ℓ（L0=0，逐级递增）。
    pub level: u32,
    /// 该级 compose 输出序号（同级别单调递增，全量/增量一致）。
    pub ordinal: u64,
}

/// 携坐标的递归走势（坐标层包装：`RMove` 结构 + 原始 K 序坐标 + 携坐标的次级别走势侧车）。
///
/// `rmove`：纯结构走势（Lean `Move` μF 镜像，descend.rs::RMove——无坐标）。
/// `start_index`/`end_index`：该走势在 L0 原始 K 序的起止（跨数据集追踪 + B2/S2 的 source_index
/// 坐标来源）。
/// `sub_moves`：构成该走势的**携坐标**次级别 `LeveledMove` 序列（`descend_leveled` 取回它们——
/// 与 `rmove` 的 `RMove::Compose.subs` 同序同长，但保留坐标）。L0 线段（递归底）的 `sub_moves` 空。
///
/// ★为什么 `sub_moves` 与 `rmove.subs` 并存（非冗余）：`rmove`（descend.rs::RMove）的 subs 是裸
/// `Vec<RMove>`（坐标剥离，Lean μF 无坐标）——`descend(rmove)` 取回的是裸 RMove，无法反查 source_index。
/// `sub_moves` 是携坐标侧车，`descend_leveled` 取回它们后 `index_of_in` 能映射回原始 K 序。两者
/// 同序同长（`sub_moves[i].rmove == rmove.subs[i]` 不变量），结构层用 rmove，坐标层用 sub_moves。
///
/// ★`sub_moves` 用 `Rc<Vec<..>>`（task #104 OOM 根因2）：递归塔逐级 `compose` 时父走势深拷贝整棵
/// 子 `sub_moves` 树（line ~231/365 的 `subs.to_vec()`），内存随级别深度膨胀（全历史 461 万 bar
/// tower 累积 = OOM 主因之一）。`Rc` 共享后 `compose` 只 clone 引用计数（廉价），子树物理上单份。
/// 不变量与 bit-exact 不受影响：`Rc<Vec<T>>` 的 `PartialEq`/`Eq` 按内容比较（deref 后逐元素），
/// `descend_leveled` clone Rc 后调用方 deref 得同一 slice。`LeveledMove` 不跨线程（无 thread::spawn /
/// rayon 消费），故 `Rc` 足够，不需 `Arc`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeveledMove {
    /// 纯结构走势（descend.rs::RMove，无坐标——Lean μF 镜像）。
    pub rmove: RMove,
    /// 该走势在 L0 原始 K 序的起点（窗口首单元起点）。
    pub start_index: usize,
    /// 该走势在 L0 原始 K 序的终点（窗口末单元终点）。
    pub end_index: usize,
    /// 构成该走势的携坐标次级别走势序列（与 `rmove` 的 Compose.subs 同序同长；L0 线段空）。
    /// `Rc` 共享（task #104）：compose 深拷贝消除——见结构体文档。
    pub sub_moves: Rc<Vec<LeveledMove>>,
    /// ★codex Q4：确定性元素身份（跨 bar 稳定，spec §13 结构映射对象身份）。
    /// 全量/增量产同 ID——`compose_level`/`compose_level_resume` 注入 ordinal。
    pub id: ElementId,
}

/// 在按 `end_index` 升序排列的走势序列中定位 `end_index == target` 的段（leftmost）。
///
/// 不变量：`moves` 按 `end_index` 升序——tower 各级 / `sub_moves` 由 `compose_level` 的连续
/// 非重叠窗口产出（`end_index` 严格递增）。partition_point 前提由 debug 断言守护（release 编译掉）。
/// O(log n) 替换 `iter().position/find(|m| m.end_index == target)` 线性扫描。
pub fn find_move_by_end_index(moves: &[LeveledMove], target: usize) -> Option<usize> {
    debug_assert!(
        moves.windows(2).all(|w| w[0].end_index <= w[1].end_index),
        "LeveledMove 序列须按 end_index 升序（partition_point 前提）"
    );
    let i = moves.partition_point(|m| m.end_index < target);
    (i < moves.len() && moves[i].end_index == target).then_some(i)
}

impl LeveledMove {
    /// L0 线段单元 → 携坐标的 `RMove::Segment`（递归底，level 0，`sub_moves` 空）。
    ///
    /// L0 走势单元是 parser 线段（`UnitRange`，有方向 + [lo,hi]）。它是递归底（`descend` 得空
    /// 序列）——`RMove::Segment { direction, lo, hi }`。坐标取线段的 source_index 区间。
    ///
    /// `id`：确定性元素身份（codex Q4）。L0 线段 ordinal = 段在 L0 序列中的索引（调用方 enumerate 注入）。
    pub fn from_unit(u: &UnitRange, id: ElementId) -> LeveledMove {
        LeveledMove {
            rmove: RMove::Segment {
                direction: u.direction,
                lo: u.lo,
                hi: u.hi,
            },
            start_index: u.start_index,
            end_index: u.end_index,
            sub_moves: Rc::new(Vec::new()),
            id,
        }
    }

    /// 上级走势单元 = 窗口内次级别 `LeveledMove` 序列 compose（`RMove::Compose`，组装-取回对偶）。
    ///
    /// `subs`：构成该上级走势的连续次级别 `LeveledMove`（窗口三段，走势分解定理二 ≥3 段）。
    /// `center`：窗口三段区间重叠真派生的中枢（`RMove::Compose.centers` 载荷）。
    /// `level`：上级走势级别（次级别 level + 1，Lean `Move.level` 严格递增/descend 递减）。
    /// `id`：确定性元素身份（codex Q4，调用方注入 ordinal = 窗口产出序）。
    /// 坐标取窗口首单元起点 + 末单元终点（上级走势覆盖其全部次级别走势的 K 序跨度）。
    ///
    /// `rmove` 的 `RMove::Compose.subs` = 窗口内次级别走势的 **rmove**（纯结构，坐标剥离——Lean μF
    /// 的 subs）；`sub_moves` 保留**携坐标**的 `subs`（坐标侧车，`descend_leveled` 取回 + `index_of_in`
    /// 映射 source_index）。两者同序同长（`sub_moves[i].rmove == rmove.subs[i]`）。子走势 `id` 继承
    /// subs 各自的 ID（已携带，非父派生）。
    pub fn compose(subs: &[LeveledMove], center: Center, level: u32, id: ElementId) -> LeveledMove {
        // ★A3 task#40 阶段0插桩（env-gated，THETA_PROFILE_STAGES 未启用时零开销直通）保留：分辨
        // compose 内部 rmove 拷贝（05c2a）vs sub_moves 侧车分配（05c2b）耗时——codex 审计要求先
        // 切分项计时再决定优化形态，不假设某一项主导。stage-0 实测（1M CL）：05c1（调用点临时数组
        // clone）/05c2a（本函数 rmove 拷贝）/05c2b（本函数 sub_moves 分配）三者耗时相当（各约
        // 05c 的 23%），非单一大头 ⟹ 两处同修：
        // (A) 调用点（compose_level/compose_level_resume）改传连续切片 `&subs_moves[i..i+3]`
        //     替代 `[a.clone(),b.clone(),c.clone()]`（win 恒为 `[i,i+1,i+2]` 连续索引，见
        //     `detect_centers_windowed{,_resume}`）——消灭 05c1 整段冗余 clone。
        // (B) `RMove::Compose.subs` 由 `Vec<RMove>` 改 `Rc<Vec<RMove>>`（descend.rs，A1 Rc 化家族
        //     延伸 #104 之后第二处）——`m.rmove.clone()`（05c2a）对 Compose 变体退化为 O(1) 引用
        //     计数（不再递归深拷贝子树），间接使 `subs.to_vec()`（05c2b，克隆 3 个 LeveledMove，
        //     其 rmove 字段现也 O(1)）一并变廉价。bit-exact：Rc<Vec<T>> 的 PartialEq/Eq/Debug 均按
        //     内容比较/打印，descend() 的 subs.as_slice()/.iter()/.first()/.last() 经自动解引用
        //     透明工作，无需改调用点。
        let sub_rmoves: Vec<RMove> = super::stage_profile::time("05c2a_rmove_clone", || {
            subs.iter().map(|m| m.rmove.clone()).collect()
        });
        let start_index = subs.first().map(|m| m.start_index).unwrap_or(0);
        let end_index = subs.last().map(|m| m.end_index).unwrap_or(0);
        let sub_moves = super::stage_profile::time("05c2b_submoves_alloc", || Rc::new(subs.to_vec()));
        LeveledMove {
            rmove: RMove::Compose {
                subs: Rc::new(sub_rmoves),
                centers: vec![center],
                level,
            },
            start_index,
            end_index,
            sub_moves,
            id,
        }
    }

    /// 该走势的方向投影（L0 线段直接取方向；上级走势取外缘趋势——次级别坐标侧车折叠为
    /// `UnitRange` 时用，与旧塔 `classify_relation` 外缘判据同源）。
    ///
    /// ★Q7 降级标注（task #145）：本函数对上级走势是 **fallback**——`project_to_units` 的方向
    /// 主来源是次级别走势类型方向（ownership Trend 块，[`center_own_dir_at`]），仅当中枢落
    /// Consolidation 块（盘整无方向）或 i==0（无入边关系）时降级到本 endpoint 比较。
    ///
    /// ★诚实有效域：上级走势的方向是**外缘占位**（首单元下移/上移），**不**冒充 §6.1 意义的
    /// 线段方向交替（上级中枢检测用几何路径 `center_from_window`，不读方向——见 center.rs）。
    pub fn fold_direction(&self, prev: Option<&LeveledMove>) -> Direction {
        match prev {
            // 首单元无前驱 ⟹ 缺省 Up（几何路径不读取，结构占位）。
            None => Direction::Up,
            // 外缘上移（hi 升）= Up，下移 = Down（与 classify_relation 外缘判据同源）。
            Some(p) => {
                if self.envelope().1 >= p.envelope().1 {
                    Direction::Up
                } else {
                    Direction::Down
                }
            }
        }
    }

    /// 外缘 `(lo, hi)` 的 **O(1)** 取回（stage 09 O(n²) 真修，本域 #H1）。
    ///
    /// `rmove.lo()/hi()`（descend.rs）对 `Compose` 变体**递归整棵子树**取 min/max——frontier 走势每 bar
    /// 重投影且其子树随窗口延伸增长 O(n) ⟹ project_to_units_resume 退化 O(n²)（400K profile 1238ms
    /// 首热点）。但外缘在 `compose` 时已算入携带的 `Center`（`center.dd/gg`）：投影契约
    /// （`project_to_units_resume` 头 + compose_level_resume line 463）保证 `units[i] = 投影(subs_moves[i])`
    /// ⟹ `center.dd = min(窗口 units.lo) = min(subs.rmove.lo()) = rmove.lo()`，`center.gg = rmove.hi()`
    /// **逐字段相等**（`detect_centers_windowed_resume` 延伸吸收 `c.dd=c.dd.min(u.lo)`/升级重切
    /// `sub_units.map(|u| u.lo).min()` 两支均聚合同一窗口 units 外缘）。故读携带 center O(1) == 递归
    /// 深扫 bit-exact。
    ///
    /// - `Segment`：外缘 = 线段自身 `[lo,hi]`（O(1) 字段读，`rmove.lo()/hi()` 对 Segment 本已 O(1)）。
    /// - `Compose`：读 `centers[0].dd/gg`（compose 恒 `vec![center]`，line 189）；缺 center（不该发生）
    ///   ⟹ 回退 `rmove.lo()/hi()`（值相同，仅慢，护 bit-exact）。
    ///
    /// mod.rs:1400 的 `debug_assert!(projected_units == 全量 project_to_units)`（走递归 `rmove.lo()/hi()`）
    /// 是本优化的**逐 bar bit-exact 神谕**：test 编译逐 bar 比对 center 读值 vs 递归深扫，任何破裂即 panic。
    #[inline]
    pub fn envelope(&self) -> (Tick, Tick) {
        match &self.rmove {
            RMove::Segment { lo, hi, .. } => (*lo, *hi),
            RMove::Compose { centers, .. } => match centers.first() {
                Some(c) => (c.dd, c.gg),
                None => (self.rmove.lo(), self.rmove.hi()),
            },
        }
    }
}

/// 从携坐标的次级别走势序列识别**canonical 中枢序列**（seed + 延伸吸收）+ 每个中枢的构成窗口。
///
/// canonical 中枢链三步构造（一类买卖点.pdf §5，第20课中心定理一，task #142）：
/// - **Step1 seed**：首个三元组经 `build` 成真中枢（L0=完整判据方向交替+核心非空；上级=几何核心非空）。
/// - **Step2 extension**：后续单元 `u_j` 区间 `[d_j,g_j] ∩ [ZD,ZG] ≠ ∅` ⟹ **同一中枢延伸**——
///   `end_index := u_j.end_index`、`DD := min(DD,d_j)`、`GG := max(GG,g_j)`；**ZD/ZG 核心冻结**
///   （现行中枢约定：核心由 seed 三段全交定，口径 B，延伸只扩外缘不改核心）。u_j 留在同一中枢，
///   不开新同级别中枢（Q2 裁决：连续围绕同一区间震荡 = 一个延伸中枢，不拆成多个重叠碎片）。
/// - **Step3 non-extension**：仅当 `d_j > ZG ∨ g_j < ZD` 停止延伸；扫描从 u_j（离开单元）继续，
///   之后才可能 seed 新同级别中枢。
///
/// 与旧「非重叠三段窗口、成立支 +3」的差异：旧版把围绕同一核心的持续震荡拆成多个外缘互相重叠的
/// 同级别中枢（L0 全历史 94.4% 相邻重叠，#141 问题包 §3-b 坐实）；canonical 版把它们吸收进一个
/// 延伸中枢——但延伸有上限（★#148 升级语义，第33课）：窗口总段数达 [`UPGRADE_TOTAL_SEGMENTS`]
/// ⟹ 整窗按每 3 段重切为本级别子中枢（核心继承 seed 三段交，外缘由子窗聚合），升级为高一级中枢
/// 经上一级 detect 自然涌现（子中枢 compose 的上级单元区间围绕同核心 ⟹ 上级 seed 高概率成立）。
/// Q2 裁决（不拆碎片）的有效域自此收窄为总段数 <9 的窗口。
///
/// `build`：中枢构造函数（L0=完整判据 `center_from_segments`；上级=几何 `center_from_window`）。
/// 返回 `Vec<(Center, (usize, usize))>`：每个中枢 + 构成它的单元闭区间 `[start, end]`（seed 三段 +
/// 延伸段，`end - start + 1 >= 3`）。
fn detect_centers_windowed(
    units: &[UnitRange],
    build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center>,
) -> Vec<(Center, (usize, usize))> {
    detect_centers_windowed_resume(units, build, 0).0
}

/// 把携坐标的走势序列规约为上级走势塔的一级（中枢序列 + 上级 `LeveledMove` 序列）。
///
/// 每个识别出的中枢由其构成窗口（连续三段次级别 `LeveledMove`）经 [`LeveledMove::compose`]
/// 封装为一个上级走势（`RMove::Compose`，subs = 窗口三段次级别 rmove，centers = [该中枢]）。
/// 这是 Lean `composeStep` 在携坐标塔上的镜像——上级走势保留构成它的次级别走势作 subs，
/// `descend` 能取回它们（旧塔折叠为无 subs 的 UnitRange，descend 得空 ⟹ B2 不可产）。
///
/// `level`：上级走势级别（本级 level + 1）。`units` 与 `subs_moves` 一一对应（同序同长——
/// `units` 是 `subs_moves` 的 `UnitRange` 投影，供 `detect_centers_windowed` 的几何/完整判据用）。
/// 返回 `(中枢序列, 上级 LeveledMove 序列)`：上级走势序列是 L(k+1) 的输入塔。
pub fn compose_level(
    units: &[UnitRange],
    subs_moves: &[LeveledMove],
    is_l0: bool,
    level: u32,
) -> (Vec<Center>, Vec<LeveledMove>) {
    let build = if is_l0 {
        super::center::center_from_segments
    } else {
        super::center::center_from_window
    };
    let windowed = detect_centers_windowed(units, build);
    let centers: Vec<Center> = windowed.iter().map(|(c, _)| *c).collect();
    // 每个中枢的构成窗口（seed 三段 + 延伸段，连续切片）→ compose 为一个上级走势。
    // ★codex Q4：确定性 ID 注入——ordinal = 窗口在该级产出序（enumerate）。全量从 0 起。
    let upper: Vec<LeveledMove> = windowed
        .iter()
        .enumerate()
        .map(|(i, (c, win))| {
            // ★task#40 fix A：win 是连续闭区间 ⟹ 直接借用连续切片，无临时数组 clone。
            // ★task#142：窗口变长（seed 三段 + 延伸段），subs = 中枢吸收的全部次级别走势。
            let subs = &subs_moves[win.0..=win.1];
            let id = ElementId { level, ordinal: i as u64 };
            LeveledMove::compose(subs, *c, level, id)
        })
        .collect();
    (centers, upper)
}

// ════════════════════════════════════════════════════════════════════════════
//  增量塔 API（task #93：解 per-bar substrate 塔构造 O(n²) 根因）
// ════════════════════════════════════════════════════════════════════════════
//
// ## 超线性根因（前序 aed4d5f5 实证）
//
// per-bar substrate 每 bar `classify_with_tower` 内部 `for level_idx in 0..=l_max`
//（mod.rs:206）每级 `compose_level` → `detect_centers_windowed` 从 `units[0..]` 全量滑窗
// 扫描。前级 confirmed 前缀稳定时重复扫描 ⟹ 超线性（classify c_exp≈2.31 主导）。
//
// ## 增量正确性（L0 纯结构证明）
//
// `detect_centers_windowed` 是**确定性左折叠**：游标 `i` 从 0 严格递增（seed 成立支消费
// `[i..=window_end]`（三段 + 延伸段）、不成立支 +1），每步决策是纯函数——seed 判定读
// `units[i..=i+2]`，延伸判定读单个 `units[j]` vs seed 冻结核心 [ZD,ZG]（不依赖其它历史）。三条推论：
//
// 1. **路径确定性**：给定 `units[0..k]`，扫描到达位置 `k` 时的游标路径与已产出 centers 序列
//    完全确定（前缀的确定性函数）。
// 2. **sealed 前缀 centers 不可变**：非末位的已产出 center 其延伸被一个显式 non-extension 单元
//    终止（判定冻结），窗口永不被后续重访（i 严格递增 ⟹ 窗口不重叠）。
// 3. **末位 center 是开放 frontier**（task #142 延伸语义）：其延伸终止于「units 用尽」而非
//    non-extension 单元时，尾部追加的新单元可延伸它 ⟹ 从 `consumed` 直接续进**不再合法**
//    （会把开放中枢误当 sealed）。唯一合法 resume 协议 = pop 末位 center + 从 `resume_from`
//    （其 seed 起点）重扫——重扫在同一确定性路径上重算该中枢并吸收新延伸段。
//
// ## 真 Fugue 547（铁律保留）
//
// 增量 compose 的 `LeveledMove::compose` 父子仍用真 sub_moves（窗口内全部次级别 LeveledMove，
// seed 三段 + 延伸段），`descend` 取回真 subs ⟹ B2/S2 真可产。增量只改"扫描从何处起"，
// 不改"compose 的 subs 来源"——subs 永远是真窗口切片（禁级别差伪造）。
//
// ## 认识论等级（formalization-validity-domain 231号）
//
// 增量等价性本身是 **L0**（纯结构，确定性左折叠的数学性质，不依赖数据）。bit-exact 逐 bar
// 断言是 **L1**（合成数据 + 真实数据管线正确性验证）。标度 exp≈1 是 **L2**（真实数据经验标度）。

/// 扫描退出断点（增量续进的锚）：while 退出时的游标位置。
///
/// `consumed` 满足 `consumed + 2 >= units.len()`（while 终止条件）。尾部追加 units 后，
/// `consumed + 2 < new_len` 可能成立 ⟹ 从 `consumed` 续扫正确（见模块文档增量证明）。
///
/// ★frontier bug 修复（task #47/#21，区间套.pdf 六~十节裁决②）：`consumed` **不能**直接作
/// resume 起点——成立支消费整个窗口后 `consumed` 越过最后一个成立窗口，把它当 sealed prefix。
/// 但该窗口尾段可能是 frontier（未确认段），新 bar 到来后（后续新段使全量扫描在此窗口后续段落
/// 产出不同中枢，或古怪线段重划改写尾段）该中枢应重算。PDF：只有**完全结束于最后 sealed 边界
/// `b_t` 前**的窗口 sealed；`b_t` 之后（含最后一个成立窗口，因其可能依赖 frontier 段）必须重算。
/// 保守版（PDF §八）：`b_t = 当前活跃候选前最后稳定端点`，rollback 重算。
/// ★task #142 延伸语义后此协议从「保守正确」升为**必需**：末位中枢在未被 non-extension 单元
/// 终止前开放（新单元可延伸它），从 `consumed` 续进恒不合法——见 `detect_centers_windowed_resume`
/// 充要条件 #1。
///
/// `resume_from` = **最后一个成立窗口的起点**（`win[0]`），即保守 `b_t` 锚。resume 从 `resume_from`
/// 重扫（而非 `consumed`）⟹ 最后一个中枢每 bar 重算，其真正 sealed（后面又出现成立窗口把它推进
/// prefix）后自然稳定。无成立窗口 ⟹ `resume_from == start_i`（无中枢可回退，续进语义不变）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WindowScanCursor {
    /// 已扫描到的游标位置（退出点；`units[..consumed]` 的扫描路径已确定）。
    pub consumed: usize,
    /// ★frontier 修复锚：本次扫描最后一个成立窗口的起点（`win[0]`）。下次 resume 起点用此
    /// （而非 `consumed`）⟹ 最后一个（frontier）中枢重算。无成立窗口 ⟹ == 本次 `start_i`
    /// （无回退，续进不变）。
    pub resume_from: usize,
    /// ★#148 升级重切：最后一个成立窗口产出的中枢数（<9 段窗口 =1；≥9 段重切窗口 =⌊n/3⌋）。
    /// frontier 回退域是**整窗产出**——调用方 pop 该数量（只 pop 1 会残留旧子中枢，与重扫
    /// 产出重复）。无成立窗口 ⟹ 0（与 `resume_from == consumed` 一致，guard 不触发 pop）。
    pub last_window_emitted: usize,
}

/// ★on2w2-cascade 读域侧车（设计 §4.1 解 A）：每个产出 center 一条，记录产出它的**窗口读域**，
/// 供 cascade 增量失效（按 `read_end_src < e` 取保留前缀 P）。与 `LevelCache.centers`/`upper_moves`
/// 1:1 对齐（同序同长，同前缀不可变 + 尾部续扫追加）。
///
/// **读域上界 `read_end_src`**（设计 §1.2.1，codex 二审终版）：detect 延伸循环停止时读了首个
/// non-extension 哨兵 `units[win.1+1]`，其源坐标才是 center 的完整读域上界（读域 `[win.0..win.1+1]`
/// ⊋ 输出区间 `[start,end]`）。`units[win.1+1]` 越界（`win.1+1 == units.len()`，窗口开放无哨兵）⟹
/// `read_end_src = usize::MAX`（+∞，永不进保留前缀，归 frontier pop 常态处理）。
///
/// **升级窗口共享**（设计 §3.5）：#148 升级重切窗口产 k 个子中枢，它们**共享同一父窗口**
/// `(win.0, win.1+1)` ⟹ `win_start`/`read_end_src` 对该窗口全部 k 个子中枢**逐值相等**。故按
/// `read_end_src < e` 的 `partition_point` 天然「整窗保留或整窗失效」——子中枢不会被从中部截断
/// （§3.5 blocker 由此自动解除，无需额外 snap 逻辑）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WinMeta {
    /// 产出该 center 的窗口起点（`win.0`，unit 下标）。升级子中枢共享父窗口起点。
    pub win_start: usize,
    /// 窗口退出点（`win.1+1` = detect 的 `j`，unit 下标）。cascade cursor 重建的 `consumed`
    /// （had_emitted_window = win_start < win_exit 判据；仅瞬态用，compose 后被 new_cursor 覆盖）。
    pub win_exit: usize,
    /// 读域上界源坐标 = `units[win.1+1].start_index`（停止哨兵）；窗口开放（无哨兵）⟹ `usize::MAX`。
    pub read_end_src: usize,
    /// 该窗口产出的 center 数（#148 升级 = k，普通 = 1）——cursor 重建时的 `last_window_emitted`。
    pub emitted: usize,
}

/// 增量窗口扫描：从 `start_i` 续扫（seed + 延伸吸收），返回新产出的 `(Center, (start,end))` 序列 +
/// 退出断点。
///
/// 与 `detect_centers_windowed(units, build)` 的关系：
/// - 全量等价：`detect_centers_windowed(units, build)` == `detect_centers_windowed_resume(units, build, 0).0`
///   （`start_i=0` 续扫 == 全量扫描）。
///
/// **bit-exact 充要条件**（调用方必须保证，否则增量破裂）：
/// 1. `start_i` 必须是一个**确定性扫描断点**，且**唯一合法取值 = 上次扫描的 `resume_from`**（最后一个
///    成立窗口的起点；无窗口时 == 退出点 `consumed`），调用方须对应 pop 最后一个已产出中枢（frontier
///    协议，见 `WindowScanCursor` 文档与 mod.rs::classify_with_tower_incremental 回退逻辑）。
///    ★延伸语义（task #142）使旧合法取值 (a)「退出点 `consumed` 直接续进」**失效**：最后一个中枢在
///    未出现 non-extension 单元前是**开放**的（尾部追加单元可延伸它），从 `consumed` 续进会把开放
///    中枢误当 sealed、对本应延伸进它的新单元开新中枢（与全量分叉）。frontier 协议天然正确：pop 开放
///    中枢 + 从其 seed 起点重扫 ⟹ 延伸在重扫中吸收新单元，bit-exact。
/// 2. `units[..start_i]` 在两次扫描间**不可变**（只允许尾部追加或 frontier 段原地改写后重扫；
///    改写落在 `start_i` 之后时无害，落在之前须调用方 cascade 全量重置）。
/// 3. 保留的前缀 centers（seed 与延伸全部结束于 `start_i` 前者）不可变——每个前缀中枢的延伸由一个
///    显式 non-extension 单元终止（该单元的判定只读该单元 vs 冻结核心，units 前缀不可变 ⟹ 判定冻结），
///    扫描永不回访其窗口。
///
/// 条件满足时，从 `start_i` 续扫产出的 tail 与全量重扫到达 `start_i` 后继续的产出逐位相同
/// （确定性左折叠：seed 判定读 `units[i..=i+2]`、延伸判定读单个 `units[j]` vs 冻结核心，均为纯函数）。
pub fn detect_centers_windowed_resume(
    units: &[UnitRange],
    build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center>,
    start_i: usize,
) -> (Vec<(Center, (usize, usize))>, Vec<WinMeta>, WindowScanCursor) {
    let mut out = Vec::new();
    // ★on2w2-cascade：与 `out` 1:1 对齐的读域侧车（每 center 一条 WinMeta）。升级子中枢共享父窗口
    // `(i, j)` ⟹ 同一 read_end_src/win_start（设计 §3.5 整窗保留/失效性质的来源）。
    let mut metas: Vec<WinMeta> = Vec::new();
    let mut i = start_i;
    // ★frontier 修复锚：最后一个成立窗口的 (起点, 产出数)。无成立窗口 ⟹ None（下面折叠为
    // (consumed, 0)，续进语义）。不能用 start_i 兜底——None 支 +1 推进后 i > start_i，会让
    // `resume_from < consumed` 误判为"有窗口产出"，导致调用方 pop 空 centers（见 mod.rs
    // had_emitted_window 守卫）。
    let mut last_window: Option<(usize, usize)> = None;
    while i + 2 < units.len() {
        match build(&units[i], &units[i + 1], &units[i + 2]) {
            Some(seed) => {
                // Step2 extension（中心定理一，PDF §5）：u_j 区间触及冻结核心 [ZD,ZG]（闭区间相交
                // `d_j <= ZG ∧ g_j >= ZD`）⟹ 同一中枢延伸——end/DD/GG 吸收，核心不动。
                // Step3 non-extension：`d_j > ZG ∨ g_j < ZD` ⟹ 停止延伸，扫描从 u_j 继续。
                let mut c = seed;
                let mut j = i + 3;
                while j < units.len() && units[j].lo <= c.zg && units[j].hi >= c.zd {
                    c.end_index = units[j].end_index;
                    c.dd = c.dd.min(units[j].lo);
                    c.gg = c.gg.max(units[j].hi);
                    j += 1;
                }
                let n = j - i;
                // ★on2w2-cascade 读域上界：停止哨兵 units[j]（首个 non-extension）的源坐标 = 完整读域
                // 上界。j==len ⟹ 窗口开放无哨兵 ⟹ +∞（usize::MAX，永不进保留前缀，设计 §1.2.1）。
                let read_end_src = units.get(j).map_or(usize::MAX, |u| u.start_index);
                let emitted = if n < UPGRADE_TOTAL_SEGMENTS {
                    // Q2 有效域（≤8 段）：一个延伸中枢，不拆碎片。
                    out.push((c, (i, j - 1)));
                    metas.push(WinMeta { win_start: i, win_exit: j, read_end_src, emitted: 1 });
                    1
                } else {
                    // ★#148 升级重切（第33课 + codex 裁定 A1/B-II/C1，codex-decide-20260704-001933）：
                    // 总段数 ≥9（本体 3 + 延伸 ≥6）⟹「构成更大级别的中枢」——整窗按每 3 段重切为
                    // 本级别子中枢（原文「每3段构成一个中枢」），余数 1-2 段并入末子中枢作延伸。
                    // 子中枢是已成立延伸中枢在第33课约定下的**重新解释**（C1：不复验 seed 判据——
                    // 延伸段仅保证各自触及核心，任意 3 段的三段交/方向交替均无保证）：核心 [ZD,ZG]
                    // 继承 seed 三段交（子中枢围绕同一核心），外缘/坐标由子窗段聚合。升级本身经塔
                    // 现有 compose 路径涌现：≥3 个围绕同核心的子中枢在上一级 detect 中 seed
                    // （center_from_window 几何判据保留最终裁决权——零跨级注入，B-III 已拒）。
                    let k = n / 3;
                    for t in 0..k {
                        let s = i + t * 3;
                        let e = if t + 1 == k { j - 1 } else { s + 2 };
                        let sub_units = &units[s..=e];
                        out.push((
                            Center {
                                zd: c.zd,
                                zg: c.zg,
                                dd: sub_units.iter().map(|u| u.lo).min().expect("子窗非空"),
                                gg: sub_units.iter().map(|u| u.hi).max().expect("子窗非空"),
                                start_index: units[s].start_index,
                                end_index: units[e].end_index,
                            },
                            (s, e),
                        ));
                        // ★升级子中枢**共享父窗口** (i, j, read_end_src, k)——设计 §3.5：k 个子中枢
                        // 逐值相同 win_start/win_exit/read_end_src ⟹ partition_point 整窗保留或整窗失效。
                        metas.push(WinMeta { win_start: i, win_exit: j, read_end_src, emitted: k });
                    }
                    k
                };
                last_window = Some((i, emitted));
                i = j;
            }
            None => {
                i += 1;
            }
        }
    }
    debug_assert_eq!(out.len(), metas.len(), "WinMeta 侧车与 center 输出 1:1 对齐");
    // 无成立窗口 ⟹ resume_from = consumed（续进，不回退）、emitted = 0；有窗口 ⟹ 该窗口起点
    // + 整窗产出数（回退域 = 整窗：升级重切窗口的全部子中枢在窗口 sealed 前均可变）。
    let (resume_from, last_window_emitted) = last_window.unwrap_or((i, 0));
    (out, metas, WindowScanCursor { consumed: i, resume_from, last_window_emitted })
}

/// 增量 compose：从 `start_i` 续扫窗口 + 把新产出的窗口 compose 为上级 `LeveledMove`。
///
/// 与 `compose_level` 的关系：
/// - 全量等价：`compose_level(units, subs, is_l0, level)` ==
///   `compose_level_resume(units, subs, is_l0, level, 0, 0)`（`.0`/`.1`/`.2` 三元组相同）。
/// - 增量：返回 `(tail_centers, tail_upper, cursor)`——tail 是新产出（追加到已缓存前缀后），
///   `cursor.consumed` 是退出断点（下次续扫起点）。
///
/// `subs_moves` 必须与 `units` 同序同长（`units` 是 `subs_moves` 的投影）。增量只追加产出，
/// **不修改**已缓存的 `LeveledMove` 前缀——真 Fugue 547：每个新 compose 的 subs 仍是真窗口切片
/// （seed 三段 + 延伸段，`subs_moves[win.0..=win.1]`，task #142），descend 取回真 subs。
///
/// ★codex Q4 确定性 ID：`prefix_count` = 已产出前缀数（调用方传 `lc.upper_moves.len()`），
/// tail ordinal = `prefix_count + i`（接续前缀，全量/增量产同 ID）。`start_i=0, prefix_count=0`
/// 时与全量 `compose_level` 逐字段 bit-identical（含 ID）。
pub fn compose_level_resume(
    units: &[UnitRange],
    subs_moves: &[LeveledMove],
    is_l0: bool,
    level: u32,
    start_i: usize,
    prefix_count: usize,
) -> (Vec<Center>, Vec<LeveledMove>, Vec<WinMeta>, WindowScanCursor) {
    let build = if is_l0 {
        super::center::center_from_segments
    } else {
        super::center::center_from_window
    };
    // ★A3 任务(a) 05 拆解插桩（env-gated，THETA_PROFILE_STAGES 未启用时零开销直通）：三子标签
    // 05a/05b/05c 分辨 detect 循环 vs tail collect vs compose build 的耗时占比；span 累加器记录
    // 续扫跨度 `units.len()-start_i`——若 avg 跨度随 n 线性增长 ⟹ H-detect（O(n²) 续扫，A4 域）；
    // O(1) ⟹ H-detect-bounded / H-clone。bit-exact：`time`/`record_span` 仅计时，不改逻辑。
    super::stage_profile::record_span("05_span", (units.len().saturating_sub(start_i)) as u64);
    let (windowed, metas, cursor) = super::stage_profile::time("05a_detect_windowed", || {
        detect_centers_windowed_resume(units, build, start_i)
    });
    let tail_centers: Vec<Center> =
        super::stage_profile::time("05b_tail_centers", || windowed.iter().map(|(c, _)| *c).collect());
    // ★A3 task#40 阶段0插桩（保留，env-gated）+ fix A/B 落地：05c1（临时数组 clone）已由直接切片
    // 消灭（见下 `&subs_moves[win[0]..win[0]+3]`），05c2 内部再分 05c2a/05c2b（compose() 内，
    // fix B 后两者均 O(1)）。三者 stage-0 实测（1M CL）曾各占 05c ~23%——非单一大头，两处同修。
    let tail_upper: Vec<LeveledMove> = super::stage_profile::time("05c_tail_upper_build", || {
        windowed
            .iter()
            .enumerate()
            .map(|(i, (c, win))| {
                // win 是连续闭区间（seed 三段 + 延伸段，task #142）——直接借用切片，不 clone 临时数组。
                let subs = &subs_moves[win.0..=win.1];
                // ★确定性 ID：tail ordinal 接续前缀（全量/增量产同 ID）。
                let id = ElementId { level, ordinal: (prefix_count + i) as u64 };
                super::stage_profile::time("05c2_compose_call", || {
                    LeveledMove::compose(subs, *c, level, id)
                })
            })
            .collect()
    });
    (tail_centers, tail_upper, metas, cursor)
}

/// 把上级 `LeveledMove` 序列投影为 `UnitRange` 序列（供下一级 `detect_centers_windowed` 的
/// 几何路径用——上级中枢检测在外缘区间上做，方向是外缘占位）。
///
/// 上级走势的 `UnitRange` = `[rmove.lo, rmove.hi]`（外缘下沿/上沿，由 subs 区间聚合，descend.rs
/// `RMove::lo/hi`）+ 坐标 + 方向。
///
/// ★Q7（一类买卖点.pdf 裁决，task #145）：「The direction of a higher-level unit = the direction
/// of the lower-level move type it represents」。`blocks` = 本级中枢链的走势类型分解（与 upper_moves
/// 1:1 的 centers 经 `decompose`，单一来源同趋势门）。单元 i 的方向：
/// - 中枢 i 按 ownership 落 **Trend(d) 块**（[`center_own_dir_at`]）⟹ 方向 = d（次级别走势类型方向）；
/// - Consolidation 块 / i==0（无入边关系）⟹ 非方向性 ⟹ **endpoint 比较降级 fallback**
///   （[`LeveledMove::fold_direction`] 外缘占位，标注：此路径的方向不携带走势类型语义，仅结构占位
///   ——盘整离开腿的方向消解归小转大/区间套语境，econ 层 XZD/Nest 通道）。
pub fn project_to_units(moves: &[LeveledMove], blocks: &[MoveBlock]) -> Vec<UnitRange> {
    moves
        .iter()
        .enumerate()
        .map(|(idx, m)| {
            let prev = if idx == 0 { None } else { Some(&moves[idx - 1]) };
            // 全量投影是 mod.rs:1400 debug_assert 神谕的一侧——保留递归 `rmove.lo()/hi()`（非热路径，
            // 每 bar 仅 test 编译调一次），作 envelope() O(1) 增量投影的独立 bit-exact 对照面。
            UnitRange {
                start_index: m.start_index,
                end_index: m.end_index,
                direction: center_own_dir_at(blocks, idx)
                    .unwrap_or_else(|| m.fold_direction(prev)),
                lo: m.rmove.lo(),
                hi: m.rmove.hi(),
            }
        })
        .collect()
}

/// ★O(n²) 真修（#106）：增量投影——`moves` 前缀不变仅尾部 append（§16）时，复用 `cache` 前缀，
/// 只对 `moves[cache.len()..]` 续投影追加。结果 bit-exact == `project_to_units(moves)`：
/// 单元 idx 的投影只依赖 `moves[idx]` + `moves[idx-1]`（`fold_direction(prev)`），前缀稳定 ⟹ 前缀投影
/// 不变；tail 的 `prev` 是已存在的前缀末元素（`moves[idx-1]`），与全量同。
///
/// **契约**：调用方保证 `moves` 前缀（`[..cache.len()]`）与上次 append 一致（cascade_reset 清空 cache
/// 后从 0 重投影 ⟹ 退化为全量，bit-exact）。前缀缩/改写 ⟹ 调用方须先 `cache.clear()`。
///
/// ★Q7 前缀稳定性：单元 i 的方向只依赖关系 R(i-1,i)（两端中枢 sealed 后标签冻结，decompose 模块头）
/// ——confirmed 前缀单元的方向跨 bar 不变；frontier 单元（临时尾关系可翻标签）由调用方
/// `truncate(prefix_count)` 每 bar 重投影覆盖（mod.rs A3 §2.5 证书化加固，先于本 Q7 存在）。
pub fn project_to_units_resume(
    moves: &[LeveledMove],
    blocks: &[MoveBlock],
    cache: &mut Vec<UnitRange>,
) {
    debug_assert!(cache.len() <= moves.len(), "投影缓存比 moves 长 ⟹ 前缀回缩未清空（违反契约）");
    for idx in cache.len()..moves.len() {
        let m = &moves[idx];
        let prev = if idx == 0 { None } else { Some(&moves[idx - 1]) };
        // ★#H1 O(n²)→O(n) 真修：`envelope()` 读携带 center O(1)，替代 `rmove.lo()/hi()` 递归深扫整棵
        // 子树（frontier 走势子树随窗口延伸 O(n)，每 bar 重投影 ⟹ O(n²)）。bit-exact 由 center.dd/gg
        // == rmove.lo()/hi() 投影契约保证 + mod.rs:1400 逐 bar debug_assert 神谕守护（见 envelope() 文档）。
        let (lo, hi) = m.envelope();
        cache.push(UnitRange {
            start_index: m.start_index,
            end_index: m.end_index,
            direction: center_own_dir_at(blocks, idx).unwrap_or_else(|| m.fold_direction(prev)),
            lo,
            hi,
        });
    }
}

/// 携坐标下钻（`descend` 的坐标层镜像）：取回构成 `parent` 的**携坐标**次级别走势序列。
///
/// `descend`（descend.rs）取回的是裸 `RMove`（坐标剥离）——无法反查 source_index。本函数取回
/// `parent.sub_moves`（携坐标侧车，与 `descend(parent.rmove)` 同序同长，`sub_moves[i].rmove ==
/// descend(parent.rmove)[i]` 不变量）。L0 线段（递归底，`sub_moves` 空）⟹ 空序列（与 descend 一致）。
pub fn descend_leveled(parent: &LeveledMove) -> Rc<Vec<LeveledMove>> {
    Rc::clone(&parent.sub_moves)
}

/// 次级别走势 `RMove` → 原始 K 序坐标（`extract_second_signals` 的 `index_of` 实现）。
///
/// `extract_second_signals` 的 `index_of: impl Fn(&RMove) -> usize` 需把 `descend parent` 取回的
/// 次级别走势映射回原始 K 序（B2/S2 的 `source_index`）。本函数在塔的坐标侧车 `subs` 里按**结构
/// 身份**（`rmove == target`）查回 `end_index`（走势终止点 = 买卖点定位端点，reference:16）。
///
/// ★坐标侧车映射（非事后查表歧义）：`subs` 是构成 `parent` 的携坐标次级别 `LeveledMove` 序列（与
/// `descend parent` 同序同长——`descend` 取回的 rmove 正是 `subs[i].rmove`）。`index_of_in` 按
/// 结构身份匹配（rmove 相等）取回对应 `LeveledMove` 的坐标。若多个次级别走势 rmove 结构全等
/// （区间+方向+subs 全同），取首个匹配——结构全等的走势在同一 parent 内坐标可不同，但 rmove
/// 身份不可区分（这是 Lean μF 无坐标的本质局限，见模块头 §still-MISSING-坐标歧义）。
pub fn index_of_in(subs: &[LeveledMove], target: &RMove) -> usize {
    subs.iter()
        .find(|m| &m.rmove == target)
        .map(|m| m.end_index)
        // 未匹配（target 不在 subs 中）⟹ 0（坐标未知占位，不冒充——调用方保证 target ∈ subs）。
        .unwrap_or(0)
}

/// 把段的 `source_index`（原始 K 序）区间映射到 `closes`/`hist` 序列的下标区间（MACD 面积坐标系）。
///
/// ★与 `signal.rs::map_src_range_to_close_idx` 同口径（坐标系一致性，formalization-validity-domain）：
/// `close_src` 是 `merged_bars` 下标 → source_index 的升序映射（`hist[k]` 对应 `close_src[k]`）。
/// 找首个 `>= start` 的下标 lo 与末个 `<= end` 的下标 hi。区间空（无 bar 落入）⟹ None。signal.rs
/// 的同名函数私有，本塔在递归组装层接入背驰需独立的坐标映射（不跨 owner 改 signal.rs 暴露私有）。
pub fn map_src_to_close_idx(close_src: &[usize], start: usize, end: usize) -> Option<(usize, usize)> {
    if start > end {
        return None;
    }
    // ponytail: partition_point 二分 O(log n) 替 position/rposition 线性 O(n)——close_src 升序保证等价
    let lo = close_src.partition_point(|&s| s < start);
    if lo >= close_src.len() {
        return None;
    }
    let hi = close_src.partition_point(|&s| s <= end);
    if hi == 0 {
        return None;
    }
    let hi = hi - 1;
    if lo > hi {
        return None;
    }
    Some((lo, hi))
}

// ═══════════════════════════════════════════════════════════════════════════════════════
//  P1 谓词闭包层（strict-nesting-divergence-plan-20260708 §P1 + strict-nesting-rulings-20260708
//  三裁决：① Cand^δ ≔ 背驰段谓词（per-level A/C 定位配对，gauge 复用 divergence.rs MacdArea
//  默认路径，严格 curr < prev）；② 盘整背驰不入链（单独诊断标志位）；③ confirm_src 为独立
//  算法确认时点。P0 D_parent 复议后，enter_src 冻结为父背驰段左端，confirm_src 仅登记延迟。
//
//  ★铁律（判据零分叉）：本层**不含任何判据代码**——判定全部经 signal.rs 同一函数
//  （`judge_first_cached`/`judge_pan_div`，仅 pub(crate) 可见性加宽，行为零改动）；prelude
//  （排序守卫/局部趋势门/first_match_idx/A 段缓存/λ_C）与 `extract_signals_with_hist_anchored`
//  逐行同构（注释见原函数 signal.rs:771-906）。事件字段全部从 judge 的入参/返回值派生 ⟹
//  ℓ0 谓词输出与 extract_signals 的 buy1/sell1 背驰确认支**构造性 bit-exact**（P1 硬门由
//  `src/bin/strict_nest_check.rs` 全量重放逐 bit 校验，不一致 ⟹ 停线报 FAIL）。
// ═══════════════════════════════════════════════════════════════════════════════════════

use super::super::types::{MoveKind, Segment, Side};
use super::decompose::{center_block_kind, center_trend_gate, decompose};
use super::divergence::{departure_move_c_start, locate_departure_move_a, DivergenceGauge};
use super::signal;

/// Cand^δ_ℓ 事件：级别 ℓ 的背驰段谓词判定（一次破中枢结构候选的完整证据包）。
///
/// 产生条件 = `judge_first_cached` 返回 `Some`（破最后中枢几何 ∧ A/C 可配对 ∧ closes 可映射，
/// P2-R2 候选口径）；`cand_delta` = D 背驰确认（默认 gauge 下 ≡ MACD 面积严格 C<A，
/// = `bits.buy1 ∨ bits.sell1`——从产出派生，无第二套判据）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandDeltaEvent {
    /// 级别 ℓ（L0=0）。
    pub level: u32,
    /// 破中枢方向侧（买侧向下破=Long，卖侧向上破=Short；= `BspPoint.struct_break_dir`）。
    pub side: Side,
    /// [新缠论] 算法确认时点（#37 P0 局部改判）：破中枢段端点 source_index。
    /// 与 `interval.1`（结构定位窗右端）是独立字段；settled 生产者上可数值相等，但不得互相派生。
    pub confirm_src: usize,
    /// [新缠论] 父级背驰段 `D_parent` = [进入 C 离开 episode 起点, seg.end_index]。
    /// 右端独立取自结构段，不从 `confirm_src` 回填。
    pub interval: (usize, usize),
    /// I(A) = [λ_A, ρ_A]（跨相邻中枢配对的前一中枢离开 episode 区间，0016:62）。
    pub a_interval: (usize, usize),
    /// `D_parent` 冻结左端：C 离开 episode 的首个同向段起点；不是确认时点。
    pub enter_src: usize,
    /// Cand^δ 真值：趋势背驰确认 D（默认 gauge ≡ 严格 C<A）。
    pub cand_delta: bool,
    /// 盘整背驰诊断标志（裁决②：**不入谓词**）——同段最近中枢 ownership 落 Consolidation 块
    /// 且 `judge_pan_div` 产证书（独立范畴，诊断并列不合取）。
    pub pan_div_diag: bool,
}

/// 级别 ℓ 的 Cand^δ 谓词提取（P1 层单一入口；入参口径与
/// [`signal::extract_signals_with_hist_anchored`] 完全一致）。
///
/// prelude 与 signal.rs full 路径逐行同构（见上方段头铁律；行为注释不在此重复）。
/// 每个破中枢结构候选产一个事件，按 (confirm_src, λ_C, λ_A) 升序。
#[allow(clippy::too_many_arguments)]
pub fn level_cand_delta(
    level: u32,
    centers: &[Center],
    segments: &[Segment],
    anchor_dirs: Option<&[Option<Direction>]>,
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
    gauge: DivergenceGauge,
) -> Vec<CandDeltaEvent> {
    // ── 以下 prelude 与 signal::extract_signals_with_hist_anchored 逐行同构 ──
    let sorted_owned: Vec<Segment>;
    let anchors_perm: Vec<Option<Direction>>;
    let (sorted, anchors_in): (&[Segment], Option<&[Option<Direction>]>) =
        if segments.windows(2).all(|w| w[0].start_index <= w[1].start_index) {
            (segments, anchor_dirs)
        } else {
            let mut idx: Vec<usize> = (0..segments.len()).collect();
            idx.sort_by_key(|&i| segments[i].start_index);
            sorted_owned = idx.iter().map(|&i| segments[i].clone()).collect();
            match anchor_dirs {
                Some(a) => {
                    anchors_perm = idx.iter().map(|&i| a[i]).collect();
                    (&sorted_owned[..], Some(&anchors_perm[..]))
                }
                None => (&sorted_owned[..], None),
            }
        };
    let anchors_self: Vec<Option<Direction>> = sorted.iter().map(|s| Some(s.direction)).collect();
    let anchors: &[Option<Direction>] = anchors_in.unwrap_or(&anchors_self);
    debug_assert_eq!(anchors.len(), sorted.len(), "anchor_dirs 与 segments 必等长");

    let centers_owned: Vec<Center>;
    let centers_sorted: &[Center] = if centers.windows(2).all(|w| w[0].end_index <= w[1].end_index)
    {
        centers
    } else {
        centers_owned = {
            let mut v = centers.to_vec();
            v.sort_by_key(|c| c.end_index);
            v
        };
        &centers_owned
    };

    let blocks = decompose(centers_sorted);
    let center_gate = center_trend_gate(centers_sorted.len(), &blocks);
    let any_trend = center_gate.iter().any(|g| g.is_some());
    let center_kind = center_block_kind(centers_sorted.len(), &blocks);
    let any_consol = center_kind.iter().any(|k| *k == Some(MoveKind::Consolidation));

    let mut first_match_idx: std::collections::HashMap<(usize, Tick, Tick), usize> =
        std::collections::HashMap::new();
    if any_trend {
        first_match_idx.reserve(centers_sorted.len());
        for (idx, c) in centers_sorted.iter().enumerate() {
            first_match_idx.entry((c.end_index, c.zd, c.zg)).or_insert(idx);
        }
    }
    let mut a_seg_cache: std::collections::HashMap<usize, Option<(usize, usize)>> =
        std::collections::HashMap::new();

    // ── 事件收集（判定全部经 signal::judge_* 同一函数，与 judge_segment 第一类支同构） ──
    let mut events: Vec<CandDeltaEvent> = Vec::new();
    for (i, seg) in sorted.iter().enumerate() {
        let Some(c_idx) = signal::nearest_confirmed_center_idx(centers_sorted, seg.start_index)
        else {
            continue;
        };
        let gate_dir = if any_trend {
            first_match_idx
                .get(&{
                    let c = &centers_sorted[c_idx];
                    (c.end_index, c.zd, c.zg)
                })
                .and_then(|&pos| center_gate[pos].map(|d| (pos, d)))
        } else {
            None
        };
        let Some((pos, dir)) = gate_dir else {
            // cert F-02（诊断可达性）：旧实现把 pan_div_diag 挂在趋势门之后，而趋势门与
            // Consolidation ownership 在同一中枢上互斥 ⟹ 诊断恒 false（死分支）。此处对
            // 「最近中枢按 ownership 属盘整块」的非趋势门段独立调用 judge_pan_div，产
            // cand_delta=false 的**纯诊断**事件：装配器基例过滤（`b.cand_delta`）与链攀升
            // （`!ev.cand_delta ⟹ continue`）双重跳过 ⟹ 结构性不入链，守住裁决②"不入链"
            // 边界；不产 BspPoint、不置一类 bit。
            if any_consol && center_kind[c_idx] == Some(MoveKind::Consolidation) {
                let c = &centers_sorted[c_idx];
                if let Some(cert) =
                    signal::judge_pan_div(c, seg, sorted, &anchors_self, hist, close_src)
                {
                    events.push(CandDeltaEvent {
                        level,
                        side: cert.side,
                        confirm_src: cert.source_index,
                        interval: cert.seg_c,
                        a_interval: cert.seg_a,
                        enter_src: cert.seg_c.0,
                        cand_delta: false,
                        pan_div_diag: true,
                    });
                }
            }
            continue; // 非趋势块 ⟹ 无第一类候选 ⟹ 无 Cand^δ 事件（谓词=第一类背驰段谓词）。
        };
        let c = &centers_sorted[c_idx];
        let prev_center = &centers_sorted[pos - 1];
        let a_seg_entry = *a_seg_cache
            .entry(c_idx)
            .or_insert_with(|| locate_departure_move_a(sorted, anchors, prev_center, c, dir));
        let c_start_entry = departure_move_c_start(sorted, anchors, c, dir, seg.start_index);
        let Some(pf) = signal::judge_first_cached(
            c, dir, seg, anchors[i], hist, dif, closes_tick, close_src, a_seg_entry,
            c_start_entry, gauge,
        ) else {
            continue; // 未破中枢/A 不可配对/不可映射 ⟹ 非结构候选（与生产路径同一 gate）。
        };
        // 事件字段全部从 judge 的入参/返回值派生（无第二套判据）：
        // judge Some ⟹ broke ∧ A 配对 ∧ 映射成立 ⟹ λ_C/I(A) 必 Some（judge 内部同断言）。
        let lambda_c = c_start_entry.expect("judge Some ⟹ λ_C Some");
        let a_interval = a_seg_entry.expect("judge Some ⟹ I(A) Some");
        let side = pf.struct_break_dir.expect("第一类结构候选必携 struct_break_dir（P2-R2 无条件置）");
        let kind_consol = any_consol && center_kind[c_idx] == Some(MoveKind::Consolidation);
        let pan_div_diag = kind_consol
            && signal::judge_pan_div(c, seg, sorted, &anchors_self, hist, close_src).is_some();
        let confirm_src = pf.source_index;
        let interval_end = seg.end_index;
        events.push(CandDeltaEvent {
            level,
            side,
            confirm_src,
            interval: (lambda_c, interval_end),
            a_interval,
            enter_src: lambda_c,
            cand_delta: pf.bits.buy1 || pf.bits.sell1,
            pan_div_diag,
        });
    }
    events.sort_by_key(|e| (e.confirm_src, e.interval.0, e.a_interval.0));
    events
}

#[cfg(test)]
mod p1_tests {
    use super::super::super::config::MacdConfig;
    use super::super::super::types::{Center, Direction, Segment, Side, Tick};
    use super::super::divergence::{compute_macd, DivergenceGauge};
    use super::super::signal::extract_signals_with_hist;
    use super::level_cand_delta;

    fn dc(zd: Tick, zg: Tick, dd: Tick, gg: Tick, ei: usize) -> Center {
        Center { zd, zg, dd, gg, start_index: 0, end_index: ei }
    }
    fn seg(direction: Direction, s: usize, e: usize, sp: Tick, ep: Tick) -> Segment {
        Segment { direction, start_index: s, end_index: e, start_price: sp, end_price: ep }
    }

    /// P1 对拍（fixture 移植自 signal.rs::first_buy_extracted_with_trend_divergence，合成数据）：
    /// ℓ0 谓词输出与 extract_signals 的 buy1/sell1 背驰确认支逐 bit 一致 + 事件字段见证。
    #[test]
    fn cand_delta_bit_exact_with_extract_signals_buy1() {
        let c0 = dc(300, 400, 290, 410, 2);
        let c1 = dc(100, 200, 90, 210, 8);
        let segs = vec![
            seg(Direction::Down, 3, 5, 350, 250),
            seg(Direction::Up, 5, 7, 250, 280),
            seg(Direction::Down, 9, 11, 150, 80),
        ];
        let prices: Vec<Tick> = vec![300, 300, 300, 300, 100, 250, 250, 250, 250, 248, 246, 244];
        let closes: Vec<f64> = prices.iter().map(|&p| p as f64).collect();
        let src: Vec<usize> = (0..prices.len()).collect();
        let series = compute_macd(&closes, &MacdConfig::default());
        let centers = [c0, c1];
        let (points, _pan) = extract_signals_with_hist(
            &centers, &segs, &series.hist, &series.dif, &prices, &src, DivergenceGauge::MacdArea,
        );
        let events = level_cand_delta(
            0, &centers, &segs, None, &series.hist, &series.dif, &prices, &src,
            DivergenceGauge::MacdArea,
        );
        // 逐 bit：buy1/sell1 背驰确认支 ⟺ cand_delta=true 事件（(src, side) 多重集相等）。
        let mut lhs: Vec<(usize, i8)> = points
            .iter()
            .flat_map(|p| {
                let mut v = Vec::new();
                if p.bits.buy1 {
                    v.push((p.source_index, 1i8));
                }
                if p.bits.sell1 {
                    v.push((p.source_index, -1i8));
                }
                v
            })
            .collect();
        let mut rhs: Vec<(usize, i8)> = events
            .iter()
            .filter(|e| e.cand_delta)
            .map(|e| (e.confirm_src, if e.side == Side::Long { 1i8 } else { -1i8 }))
            .collect();
        lhs.sort_unstable();
        rhs.sort_unstable();
        assert!(!lhs.is_empty(), "fixture 必产 buy1（非空对拍）");
        assert_eq!(lhs, rhs, "P1 铁律：谓词 cand_delta 与 buy1/sell1 背驰确认支逐 bit 一致");
        assert_eq!(events.len(), 1, "唯一破中枢结构候选（C 段）");
        let e = &events[0];
        assert!(e.cand_delta, "C<A 背驰确认 ⟹ Cand^δ=true");
        assert_eq!(e.side, Side::Long);
        assert_eq!(e.confirm_src, 11, "确认时点=完成时（破中枢段端点，裁决③）");
        assert_eq!(e.interval, (9, 11), "I(C) = [λ_C, seg.end]（Q5 区间口径）");
        assert_eq!(e.a_interval, (3, 5), "I(A) = 前中枢离开 episode");
        assert_eq!(e.enter_src, 9, "D_parent 左端 = C 离开 episode 起点");
        assert!(!e.pan_div_diag, "趋势路径无盘整背驰诊断（裁决②不入链）");
    }

    /// cert F-02 回归：非趋势门段（最近中枢按 ownership 落 Consolidation 块）的盘整背驰诊断
    /// 独立可达——fixture 移植自 signal.rs::pan_div_cert_emitted_in_consolidation_block_zero_
    /// first_class_bits。修复前 pan_div_diag 挂在趋势门之后，与盘整 ownership 在同一中枢上
    /// 互斥 ⟹ 恒 false 死分支；修复后产恰一条 cand_delta=false 的**纯诊断**事件（装配器基例
    /// 过滤与链攀升双重跳过 ⟹ 结构性不入链，守住裁决②边界）。
    #[test]
    fn pan_div_diag_reachable_in_consolidation_without_trend_gate() {
        let c0 = dc(100, 200, 90, 210, 2);
        let c1 = dc(300, 400, 290, 410, 5); // c0→c1 上涨（趋势块）
        let c2 = dc(350, 450, 250, 460, 8); // c1→c2 扩张 ⟹ c2 按 ownership 落盘整块
        let segs = vec![
            seg(Direction::Down, 9, 11, 460, 330),  // A：第一次离开（330 < zd=350 破核心）
            seg(Direction::Up, 11, 13, 330, 380),   // 回中枢段（380 ≥ 350 回核心内侧）
            seg(Direction::Down, 13, 15, 380, 300), // C：第二次离开破核心（C<A 背驰）
        ];
        let prices: Vec<Tick> = vec![
            100, 100, 100, 100, 60, 140, 100, 95, 105, 105, 60, 90, 95, 93, 91, 89,
        ];
        let closes: Vec<f64> = prices.iter().map(|&p| p as f64).collect();
        let src: Vec<usize> = (0..prices.len()).collect();
        let series = compute_macd(&closes, &MacdConfig::default());
        let centers = [c0, c1, c2];
        let events = level_cand_delta(
            0, &centers, &segs, None, &series.hist, &series.dif, &prices, &src,
            DivergenceGauge::default(),
        );
        // 恰一条纯诊断事件；零 cand_delta=true（诊断不入谓词）。
        assert_eq!(events.len(), 1, "盘整块内恰一张 PanDivCert ⟹ 恰一条诊断事件");
        let e = &events[0];
        assert!(e.pan_div_diag, "cert F-02：盘整背驰诊断可达（修复前死分支恒 false）");
        assert!(
            !e.cand_delta,
            "裁决②：盘整背驰不入谓词 ⟹ cand_delta=false（装配器双重跳过 ⟹ 不入链）"
        );
        assert_eq!(e.side, Side::Long, "向下破 ⟹ Long 候选（仅诊断标注）");
        assert_eq!(e.confirm_src, 15, "因果触发点 = 破中枢段端点");
        assert_eq!(e.interval, (13, 15), "I(C) = 当前离开走势区间");
        assert_eq!(e.a_interval, (9, 11), "I(A) = 前一次同向离开末段");
        assert_eq!(e.enter_src, 13, "enter_src = λ_C = I(C) 起点");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::descend::descend;
    use super::super::super::types::Tick;

    fn unit(si: usize, ei: usize, dir: Direction, lo: Tick, hi: Tick) -> UnitRange {
        UnitRange { start_index: si, end_index: ei, direction: dir, lo, hi }
    }

    fn up() -> Direction { Direction::Up }
    fn down() -> Direction { Direction::Down }

    /// 测试用 ID 生成器（codex Q4：确定性 ElementId）。
    fn eid(level: u32, ordinal: u64) -> ElementId {
        ElementId { level, ordinal }
    }

    /// 测试用 from_unit 包装（自动注入 L0 ID，ordinal 按调用序由调用方给）。
    fn from_unit(u: &UnitRange, ordinal: u64) -> LeveledMove {
        LeveledMove::from_unit(u, eid(0, ordinal))
    }

    /// 测试用 compose 包装（自动注入上级 ID）。
    fn compose(subs: &[LeveledMove], center: Center, level: u32, ordinal: u64) -> LeveledMove {
        LeveledMove::compose(subs, center, level, eid(level, ordinal))
    }

    /// L0 线段单元 → RMove::Segment（递归底，坐标保留）。
    #[test]
    fn from_unit_is_segment_with_coords() {
        let u = unit(4, 8, down(), 90, 150);
        let lm = LeveledMove::from_unit(&u, eid(0, 0));
        assert_eq!(lm.rmove, RMove::Segment { direction: down(), lo: 90, hi: 150 });
        assert_eq!((lm.start_index, lm.end_index), (4, 8));
        // 递归底：descend 得空（L0 线段无次级别）。
        assert!(descend(&lm.rmove).is_empty());
    }

    /// ★组装-取回对偶（descend ∘ compose = id）：compose 三段次级别 → descend 取回它们的 rmove。
    #[test]
    fn compose_descend_roundtrip_preserves_subs() {
        let s0 = LeveledMove::from_unit(&unit(0, 4, up(), 0, 10), eid(0, 0));
        let s1 = LeveledMove::from_unit(&unit(4, 8, down(), 3, 12), eid(0, 1));
        let s2 = LeveledMove::from_unit(&unit(8, 12, up(), 5, 15), eid(0, 2));
        let c = Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: 0, end_index: 12 };
        let parent = LeveledMove::compose(&[s0.clone(), s1.clone(), s2.clone()], c, 1, eid(1, 0));
        // descend 取回三段次级别 rmove（旧塔 UnitRange 折叠后 descend 得空）。
        let subs = descend(&parent.rmove);
        assert_eq!(subs.to_vec(), vec![s0.rmove.clone(), s1.rmove.clone(), s2.rmove.clone()]);
        // 上级走势坐标 = 窗口首起点..末终点。
        assert_eq!((parent.start_index, parent.end_index), (0, 12));
        // 级别 = 1（次级别 segment level 0 + 1）。
        assert_eq!(parent.rmove.level(), 1);
    }

    /// detect_centers_windowed 返回中枢 + 构成窗口索引（窗口封装 compose 的来源）。
    #[test]
    fn windowed_returns_center_and_window_indices() {
        // 三段重叠成中枢（几何路径）：[0,10],[3,12],[5,15] → 中枢 [5,10]。
        let units = vec![
            unit(0, 4, up(), 0, 10),
            unit(4, 8, down(), 3, 12),
            unit(8, 12, up(), 5, 15),
        ];
        let windowed = detect_centers_windowed(&units, super::super::center::center_from_window);
        assert_eq!(windowed.len(), 1);
        let (c, win) = &windowed[0];
        assert_eq!((c.zd, c.zg), (5, 10));
        assert_eq!(*win, (0, 2)); // 构成窗口闭区间（seed 三段，无延伸段）
    }

    /// ★compose_level：三段 L0 线段 → 一个上级走势（RMove::Compose 携 subs）。
    #[test]
    fn compose_level_l0_yields_compose_with_subs() {
        // 完整判据 L0：上-下-上 + 全三段核心非空（lo<=hi 不变量，segment_to_unit 已规约）。
        let units = vec![
            unit(0, 4, up(), 0, 10),
            unit(4, 8, down(), 3, 12),
            unit(8, 12, up(), 5, 15),
        ];
        let moves: Vec<LeveledMove> = units.iter().enumerate().map(|(i, u)| LeveledMove::from_unit(u, eid(0, i as u64))).collect();
        let (centers, upper) = compose_level(&units, &moves, true, 1);
        assert_eq!(centers.len(), 1, "上-下-上 全三段核心非空 ⟹ 一个中枢");
        assert_eq!(upper.len(), 1, "一个中枢 ⟹ 一个上级走势");
        // 上级走势是 RMove::Compose，descend 取回构成它的三段 L0 线段。
        let subs = descend(&upper[0].rmove);
        assert_eq!(subs.len(), 3, "上级走势 descend 取回三段次级别走势（B2 可产的前提）");
        assert_eq!(subs.to_vec(), vec![moves[0].rmove.clone(), moves[1].rmove.clone(), moves[2].rmove.clone()]);
    }

    /// index_of_in：从坐标侧车按结构身份查回次级别走势的 end_index。
    #[test]
    fn index_of_maps_rmove_to_source_index() {
        let s0 = LeveledMove::from_unit(&unit(0, 4, up(), 0, 10), eid(0, 0));
        let s1 = LeveledMove::from_unit(&unit(4, 8, down(), 3, 12), eid(0, 1));
        let subs = vec![s0.clone(), s1.clone()];
        // 按结构身份查 s1.rmove → end_index=8。
        assert_eq!(index_of_in(&subs, &s1.rmove), 8);
        assert_eq!(index_of_in(&subs, &s0.rmove), 4);
        // 未匹配 ⟹ 0（占位，调用方保证 target ∈ subs）。
        let alien = RMove::Segment { direction: up(), lo: 99, hi: 100 };
        assert_eq!(index_of_in(&subs, &alien), 0);
    }

    /// project_to_units：上级走势序列 → UnitRange 序列（供下一级几何检测 + 坐标传递）。
    #[test]
    fn project_preserves_outer_envelope_and_coords() {
        let s0 = LeveledMove::from_unit(&unit(0, 4, up(), 0, 10), eid(0, 0));
        let s1 = LeveledMove::from_unit(&unit(4, 8, down(), 3, 12), eid(0, 1));
        let s2 = LeveledMove::from_unit(&unit(8, 12, up(), 5, 15), eid(0, 2));
        let c = Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: 0, end_index: 12 };
        let parent = LeveledMove::compose(&[s0, s1, s2], c, 1, eid(1, 0));
        let units = project_to_units(&[parent.clone()], &[]); // 无块信息 ⟹ Q7 fallback（本测试只验坐标）
        assert_eq!(units.len(), 1);
        // 外缘 = subs 区间聚合（lo=min=0, hi=max=15）。
        assert_eq!((units[0].lo, units[0].hi), (0, 15));
        // 坐标保留。
        assert_eq!((units[0].start_index, units[0].end_index), (0, 12));
        // 首单元无前驱 ⟹ 方向缺省 Up（几何路径不读取）。
        assert_eq!(units[0].direction, up());
    }

    // ──────────────────────────────────────────────────────────────────────
    //  增量塔 API bit-exact（task #93：resume == 全量，逐位等价）
    // ──────────────────────────────────────────────────────────────────────

    /// ★增量基元 bit-exact：`detect_centers_windowed_resume(.., 0)` == `detect_centers_windowed`。
    #[test]
    fn resume_from_zero_equals_full_scan() {
        let units: Vec<UnitRange> = (0..9)
            .map(|i| {
                let dir = if i % 2 == 0 { up() } else { down() };
                unit(i * 4, i * 4 + 4, dir, 0, 100)
            })
            .collect();
        let full = detect_centers_windowed(&units, super::super::center::center_from_segments);
        let (res, _metas, cursor) =
            detect_centers_windowed_resume(&units, super::super::center::center_from_segments, 0);
        assert_eq!(res.len(), full.len(), "start_i=0 续扫产出 == 全量");
        for (r, f) in res.iter().zip(full.iter()) {
            assert_eq!(r.0, f.0, "中枢相等");
            assert_eq!(r.1, f.1, "窗口索引相等");
        }
        // 9 段 → 3 窗口（成立支 +3 三次）⟹ consumed = 9（9+2 >= 9 退出）。
        assert_eq!(cursor.consumed, 9, "9 段全消费，退出断点 consumed=9");
    }

    /// ★增量核心 bit-exact：尾部追加后按 frontier 协议续扫 == 全量重扫。
    ///
    /// 9 段全重叠（★#148 升级语义：总段数达 9 ⟹ 整窗重切 3 个子中枢），前 6 段先扫（<9，
    /// 1 个延伸中枢）→ 追加 3 段跨越升级阈值。唯一合法 resume 协议（task #142 充要条件 #1
    /// + #148 整窗回退）：pop 末窗口全部产出（`last_window_emitted`）+ 从 `resume_from` 重扫
    /// （窗口开放期间其产出的值与**数量**均可变——本例 1 个变 3 个）。
    #[test]
    fn resume_after_append_matches_full_rescan() {
        // 9 段交替全重叠，分两批：前 6 段 → 追加 3 段。
        let all_units: Vec<UnitRange> = (0..9)
            .map(|i| {
                let dir = if i % 2 == 0 { up() } else { down() };
                unit(i * 4, i * 4 + 4, dir, 0, 100)
            })
            .collect();
        let build = super::super::center::center_from_segments;

        // 全量基准（升级重切：seed [0..=2] 吸收 3..=8 后 n=9 ⟹ 3 个子中枢，每 3 段一个）。
        let full = detect_centers_windowed(&all_units, build);
        assert_eq!(full.len(), 3, "9 段全重叠 = 升级重切 3 个子中枢（第33课，#148）");
        assert_eq!(full[0].1, (0, 2), "子窗1 = seed 三段");
        assert_eq!(full[1].1, (3, 5), "子窗2");
        assert_eq!(full[2].1, (6, 8), "子窗3");

        // 增量：前 6 段先扫（n=6 < 9 ⟹ 1 个开放延伸中枢 (0,5)）。
        let (mut prefix, _m6, cursor6) = detect_centers_windowed_resume(&all_units[..6], build, 0);
        assert_eq!(cursor6.last_window_emitted, 1, "6 段窗口产出 1 个中枢");
        // frontier 协议：pop 末窗口全部产出 + 从 resume_from（窗口起点）重扫。
        if cursor6.resume_from < cursor6.consumed {
            prefix.truncate(prefix.len() - cursor6.last_window_emitted);
        }
        let (tail, _m9, cursor9) =
            detect_centers_windowed_resume(&all_units, build, cursor6.resume_from);
        assert_eq!(cursor9.last_window_emitted, 3, "重扫后末窗口产出 3 个子中枢");

        // 拼接 == 全量（pop 1 产 3：窗口产出数量跨阈值改变）。
        let mut combined = prefix.clone();
        combined.extend(tail);
        assert_eq!(combined.len(), full.len(), "增量拼接长度 == 全量");
        for (c, f) in combined.iter().zip(full.iter()) {
            assert_eq!(c.0, f.0, "中枢相等");
            assert_eq!(c.1, f.1, "窗口索引相等");
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★#148 升级重切语义（第33课 + codex 裁定 A1/B-II/C1，codex-decide-20260704-001933）
    // ──────────────────────────────────────────────────────────────────────

    /// 触核心但外缘参差的 9+ 段窗口构造器：seed 三段交 = [40,60]（交替 up/down），
    /// 延伸段全部触及 [40,60] 但外缘逐段变化（验证子窗外缘聚合与核心继承的区分）。
    fn upgrade_units(n: usize) -> Vec<UnitRange> {
        (0..n)
            .map(|i| {
                let dir = if i % 2 == 0 { up() } else { down() };
                // seed 三段 [40-i, 60+i] ⟹ 三段交 = [40,60]；延伸段 [30+i, 70+i]（触核心）。
                if i < 3 {
                    unit(i * 4, i * 4 + 4, dir, 40 - i as Tick, 60 + i as Tick)
                } else {
                    unit(i * 4, i * 4 + 4, dir, 30 + i as Tick, 70 + i as Tick)
                }
            })
            .collect()
    }

    /// Q2 有效域边界：8 段（seed 3 + 延伸 5）仍是一个延伸中枢——「延伸不能超过5段」的
    /// 允许上限，不重切。
    #[test]
    fn eight_segment_window_stays_single_extension_center() {
        let units = upgrade_units(8);
        let windowed = detect_centers_windowed(&units, super::super::center::center_from_segments);
        assert_eq!(windowed.len(), 1, "8 段 = Q2 域内一个延伸中枢");
        assert_eq!(windowed[0].1, (0, 7));
        let c = windowed[0].0;
        // seed 三段 [40,60],[39,61],[38,62] ⟹ 口径 B 三段交 = [max(lo),min(hi)] = [40,60]。
        assert_eq!((c.zd, c.zg), (40, 60), "核心 = seed 三段交（口径 B 冻结）");
    }

    /// 升级阈值：9 段 ⟹ 恰 3 个子中枢。核心继承 seed 三段交（C1：重新解释不复验 seed），
    /// 外缘由各子窗段聚合，坐标 = 子窗首起点..末终点。
    #[test]
    fn nine_segment_window_splits_into_three_subcenters() {
        let units = upgrade_units(9);
        let windowed = detect_centers_windowed(&units, super::super::center::center_from_segments);
        assert_eq!(windowed.len(), 3, "9 段 = 3 个子中枢（每 3 段一个）");
        let core = (windowed[0].0.zd, windowed[0].0.zg);
        // seed 三段 [40,60],[39,61],[38,62] ⟹ 口径 B 三段交 = [40,60]。
        assert_eq!(core, (40, 60), "核心 = seed 三段交");
        for (t, (c, win)) in windowed.iter().enumerate() {
            assert_eq!(*win, (t * 3, t * 3 + 2), "子窗 t 覆盖段 [3t, 3t+2]");
            assert_eq!((c.zd, c.zg), core, "全部子中枢继承同一冻结核心");
            // 外缘 = 子窗三段 lo/hi 聚合。
            let dd = units[t * 3..=t * 3 + 2].iter().map(|u| u.lo).min().unwrap();
            let gg = units[t * 3..=t * 3 + 2].iter().map(|u| u.hi).max().unwrap();
            assert_eq!((c.dd, c.gg), (dd, gg), "外缘 = 子窗段聚合");
            assert_eq!(c.start_index, units[t * 3].start_index);
            assert_eq!(c.end_index, units[t * 3 + 2].end_index);
        }
    }

    /// 余数并入末子中枢：10/11 段 ⟹ 仍 3 个子中枢（末子窗 4/5 段）；12 段 ⟹ 4 个。
    #[test]
    fn remainder_segments_merge_into_last_subcenter() {
        let build = super::super::center::center_from_segments;
        for (n, expect_k, last_win) in [(10, 3, (6, 9)), (11, 3, (6, 10)), (12, 4, (9, 11))] {
            let units = upgrade_units(n);
            let windowed = detect_centers_windowed(&units, build);
            assert_eq!(windowed.len(), expect_k, "{n} 段 ⟹ ⌊n/3⌋={expect_k} 个子中枢");
            assert_eq!(windowed[expect_k - 1].1, last_win, "{n} 段末子窗吸收余数");
        }
    }

    /// ★升级涌现（B-II 全链）：9 段重切的 3 个子中枢 compose 为上级单元后，上一级
    /// detect（center_from_window 几何判据）seed 成立 ⟹ 高一级中枢诞生——升级经塔现有
    /// compose 路径自然涌现，零跨级注入。
    #[test]
    fn upgraded_subcenters_seed_higher_level_center() {
        let units = upgrade_units(9);
        let moves: Vec<LeveledMove> =
            units.iter().enumerate().map(|(i, u)| from_unit(u, i as u64)).collect();
        // 本级：9 段 → 3 子中枢 → 3 个上级走势单元。
        let (centers, upper) = compose_level(&units, &moves, true, 1);
        assert_eq!(centers.len(), 3);
        assert_eq!(upper.len(), 3);
        // descend 取回真 subs（真 Fugue：子窗切片 3 段）。
        for u in &upper {
            assert_eq!(descend(&u.rmove).len(), 3, "子中枢 subs = 子窗真三段");
        }
        // 上一级：3 个子中枢单元投影后 detect ⟹ 高一级中枢 seed 成立（围绕同核心，区间交非空）。
        let blocks = super::super::decompose::decompose(&centers);
        let l1_units = project_to_units(&upper, &blocks);
        let (l1_centers, _) = compose_level(&l1_units, &upper, false, 2);
        assert_eq!(l1_centers.len(), 1, "升级涌现：高一级中枢由子中枢重叠自然 seed（第33课）");
    }

    /// ★compose_level_resume bit-exact：全量 `compose_level` == `compose_level_resume(.., 0)`，
    /// 且增量追加后 `(prefix ++ tail)` == 全量（centers + upper LeveledMove 序列逐位相等）。
    #[test]
    fn compose_level_resume_matches_full_compose() {
        let units: Vec<UnitRange> = (0..9)
            .map(|i| {
                let dir = if i % 2 == 0 { up() } else { down() };
                unit(i * 4, i * 4 + 4, dir, 0, 100)
            })
            .collect();
        let moves: Vec<LeveledMove> = units.iter().enumerate().map(|(i, u)| LeveledMove::from_unit(u, eid(0, i as u64))).collect();

        // 全量 compose_level。
        let (full_c, full_u) = compose_level(&units, &moves, true, 1);

        // resume from 0 == 全量。
        let (rc, ru, _, _) = compose_level_resume(&units, &moves, true, 1, 0, 0);
        assert_eq!(rc, full_c, "resume(0) centers == 全量");
        assert_eq!(ru, full_u, "resume(0) upper == 全量");

        // 增量：前 6 段 compose（产出 1 个开放中枢 + 其上级走势）。
        let (mut pc, mut pu, _m6, cursor6) = compose_level_resume(&units[..6], &moves[..6], true, 1, 0, 0);
        // frontier 协议（task #142 充要条件 #1）：pop 末位开放中枢及其上级走势 + 从 resume_from 重扫。
        if cursor6.resume_from < cursor6.consumed {
            pc.pop();
            pu.pop();
        }
        let (tc, tu, _mt, _) =
            compose_level_resume(&units, &moves, true, 1, cursor6.resume_from, pu.len());

        let mut comb_c = pc.clone();
        comb_c.extend(tc);
        let mut comb_u = pu.clone();
        comb_u.extend(tu);
        assert_eq!(comb_c, full_c, "增量拼接 centers == 全量");
        assert_eq!(comb_u, full_u, "增量拼接 upper == 全量（真 subs LeveledMove）");
    }

    /// ★边界：不成立支前缀的增量。前段不组中枢（方向不交替）⟹ consumed 逐段 +1 推进，
    /// 追加后从 consumed 续扫仍 == 全量。
    #[test]
    fn resume_with_non_matching_prefix_advances_by_one() {
        // 前 2 段同向（不交替，不成立支 +1 推进），第 3-5 段交替组中枢。
        let units = vec![
            unit(0, 4, up(), 0, 10),
            unit(4, 8, up(), 5, 15),     // 同向，与 [0] 不交替
            unit(8, 12, down(), 3, 12),  // 与 [1] 交替
            unit(12, 16, up(), 5, 15),   // 与 [2] 交替 → [1,2,3] 组中枢
            unit(16, 20, down(), 4, 11),
        ];
        let build = super::super::center::center_from_segments;
        let full = detect_centers_windowed(&units, build);

        // 前 2 段：不成立支，i: 0→1→2（2+2>=2 退出，consumed=2，但 len=2 时 0+2<2 假 ⟹ 不进循环，
        // consumed=0）。实际 units[..2] 长度 2，while 0+2<2 假 ⟹ consumed=0。
        // 这验证空扫描也正确返回断点。
        let (prefix, _mp, c0) = detect_centers_windowed_resume(&units[..2], build, 0);
        assert!(prefix.is_empty(), "2 段凑不齐窗口 ⟹ 空产出");
        assert_eq!(c0.consumed, 0, "len=2 不进 while ⟹ consumed=0");
        // 追加到 5 段从 consumed=0 续扫 == 全量。
        let (tail, _mt, _) = detect_centers_windowed_resume(&units, build, c0.consumed);
        assert_eq!(tail.len(), full.len(), "从 0 续扫 == 全量");
        for (t, f) in tail.iter().zip(full.iter()) {
            assert_eq!(t.0, f.0);
            assert_eq!(t.1, f.1);
        }
    }

    /// 多级递归塔：L0 → L1 → L2，每级 descend 取回下级走势（级别严格递减）。
    #[test]
    fn multi_level_tower_descend_decreases_level() {
        // 9 段 L0 三组「核心分离、外缘重叠」（task #142 延伸语义：全重叠 fixture 会被吸收为
        // 1 个延伸中枢，多级塔须组间 non-extension 分离——首段 d>ZG 终止前组延伸；
        // 三组外缘共同重叠 [152,180] 非空 ⟹ L2 几何中枢成立）。
        let ranges = [
            (0, 50), (40, 200), (40, 50),        // 组1 核心 [40,50]，外缘 [0,200]
            (55, 150), (52, 180), (55, 160),     // 组2 核心 [55,150]（首段 55>50 non-ext），外缘 [52,180]
            (155, 300), (152, 280), (155, 290),  // 组3 核心 [155,280]（首段 155>150 non-ext），外缘 [152,300]
        ];
        let units: Vec<UnitRange> = ranges
            .iter()
            .enumerate()
            .map(|(i, &(lo, hi))| {
                let dir = if i % 2 == 0 { up() } else { down() };
                unit(i * 4, i * 4 + 4, dir, lo, hi)
            })
            .collect();
        let moves: Vec<LeveledMove> = units.iter().enumerate().map(|(i, u)| LeveledMove::from_unit(u, eid(0, i as u64))).collect();
        // L0 → L1（完整判据，方向交替）。
        let (_c1, l1) = compose_level(&units, &moves, true, 1);
        assert_eq!(l1.len(), 3, "三组核心分离 → 3 个中枢 → 3 个 L1 走势");
        for m in &l1 {
            assert_eq!(m.rmove.level(), 1);
            // L1 走势 descend 取回 3 段 L0 线段（level 0）。
            for sub in descend(&m.rmove) {
                assert_eq!(sub.level(), 0, "L1 descend 得 L0 线段");
            }
        }
        // L1 → L2（几何路径）。
        let l1_units = project_to_units(&l1, &[]); // 无块信息 ⟹ Q7 fallback（本测试只验区间聚合）
        let (_c2, l2) = compose_level(&l1_units, &l1, false, 2);
        assert_eq!(l2.len(), 1, "3 个 L1 走势 → 1 窗口 → 1 个 L2 走势");
        assert_eq!(l2[0].rmove.level(), 2);
        // L2 走势 descend 取回 3 个 L1 走势（level 1）。
        for sub in descend(&l2[0].rmove) {
            assert_eq!(sub.level(), 1, "L2 descend 得 L1 走势");
        }
    }

    /// ★#106 O(n²) 真修守卫：`project_to_units_resume` 逐批追加 == `project_to_units` 全量（逐字段）。
    /// 失败 ⟹ 增量投影破坏 bit-exact（下一级 units 输入与全量发散 ⟹ 整塔判定漂移）。
    #[test]
    fn project_to_units_resume_matches_full() {
        let moves: Vec<LeveledMove> = (0..9)
            .map(|i| {
                let dir = if i % 2 == 0 { up() } else { down() };
                // lo/hi 各异（非全 [0,100]）⟹ 验证投影确实读 rmove.lo()/hi() 而非常量。
                let u = unit(i * 4, i * 4 + 4, dir, i as i64, 100 - i as i64);
                LeveledMove::from_unit(&u, eid(0, i as u64))
            })
            .collect();

        let full = project_to_units(&moves, &[]);

        // 三批增量追加：[..4] → [..7] → 全 9（前缀不变仅尾部 append）。
        let mut cache: Vec<UnitRange> = Vec::new();
        project_to_units_resume(&moves[..4], &[], &mut cache);
        assert_eq!(&cache[..], &full[..4], "首批 4 == 全量前缀");
        project_to_units_resume(&moves[..7], &[], &mut cache);
        assert_eq!(&cache[..], &full[..7], "次批 7 == 全量前缀");
        project_to_units_resume(&moves, &[], &mut cache);
        assert_eq!(cache, full, "全量追加后 == project_to_units 全量（逐字段）");

        // cascade_reset 退化：清空后从 0 重投影 == 全量。
        cache.clear();
        project_to_units_resume(&moves, &[], &mut cache);
        assert_eq!(cache, full, "清空重投影（cascade_reset 路径）== 全量");
    }

    /// Q7 覆盖见证（codex ac4-r2 Q7 #3：空 blocks 测试只锁 fallback，须锁「块方向覆盖 endpoint」）：
    /// fold_direction（外缘 hi 递减 ⟹ Down）与 ownership Trend(Up) 块相反 ⟹ 方向取块方向 Up；
    /// i==0（无入边关系）降级 fallback（prev=None ⟹ Up 占位）；full 与 resume 逐字段一致。
    #[test]
    fn project_to_units_block_dir_overrides_endpoint_fallback() {
        use super::super::decompose::MoveStatus;
        use super::super::super::types::MoveKind;
        // hi 严格递减（100,99,98）⟹ fold_direction 对 i≥1 给 Down。
        let moves: Vec<LeveledMove> = (0..3)
            .map(|i| {
                let u = unit(i * 4, i * 4 + 4, down(), 0, 100 - i as i64);
                LeveledMove::from_unit(&u, eid(0, i as u64))
            })
            .collect();
        let blocks = [MoveBlock {
            start_center: 0,
            end_center: 2,
            kind: MoveKind::Trend,
            dir: Some(Direction::Up),
            status: MoveStatus::Active,
        }];
        let full = project_to_units(&moves, &blocks);
        assert_eq!(full[0].direction, Direction::Up, "i=0 无入边关系 ⟹ fallback（prev=None 占位 Up）");
        assert_eq!(full[1].direction, Direction::Up, "Trend(Up) 块方向覆盖 endpoint Down");
        assert_eq!(full[2].direction, Direction::Up, "Trend(Up) 块方向覆盖 endpoint Down");
        // 对照：无块 ⟹ endpoint fallback 给 Down（证明上面的 Up 确实来自块，非 fold 巧合）。
        let no_blocks = project_to_units(&moves, &[]);
        assert_eq!(no_blocks[1].direction, Direction::Down, "对照：空 blocks ⟹ endpoint Down");
        // resume 与 full 逐字段一致（含块方向路径）。
        let mut cache: Vec<UnitRange> = Vec::new();
        project_to_units_resume(&moves[..2], &blocks, &mut cache);
        project_to_units_resume(&moves, &blocks, &mut cache);
        assert_eq!(cache, full, "resume（块方向路径）== 全量");
    }
}
