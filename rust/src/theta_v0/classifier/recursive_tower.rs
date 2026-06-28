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

use super::super::types::{Center, Direction};
use super::center::UnitRange;
use super::descend::RMove;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeveledMove {
    /// 纯结构走势（descend.rs::RMove，无坐标——Lean μF 镜像）。
    pub rmove: RMove,
    /// 该走势在 L0 原始 K 序的起点（窗口首单元起点）。
    pub start_index: usize,
    /// 该走势在 L0 原始 K 序的终点（窗口末单元终点）。
    pub end_index: usize,
    /// 构成该走势的携坐标次级别走势序列（与 `rmove` 的 Compose.subs 同序同长；L0 线段空）。
    pub sub_moves: Vec<LeveledMove>,
}

impl LeveledMove {
    /// L0 线段单元 → 携坐标的 `RMove::Segment`（递归底，level 0，`sub_moves` 空）。
    ///
    /// L0 走势单元是 parser 线段（`UnitRange`，有方向 + [lo,hi]）。它是递归底（`descend` 得空
    /// 序列）——`RMove::Segment { direction, lo, hi }`。坐标取线段的 source_index 区间。
    pub fn from_unit(u: &UnitRange) -> LeveledMove {
        LeveledMove {
            rmove: RMove::Segment {
                direction: u.direction,
                lo: u.lo,
                hi: u.hi,
            },
            start_index: u.start_index,
            end_index: u.end_index,
            sub_moves: Vec::new(),
        }
    }

    /// 上级走势单元 = 窗口内次级别 `LeveledMove` 序列 compose（`RMove::Compose`，组装-取回对偶）。
    ///
    /// `subs`：构成该上级走势的连续次级别 `LeveledMove`（窗口三段，走势分解定理二 ≥3 段）。
    /// `center`：窗口三段区间重叠真派生的中枢（`RMove::Compose.centers` 载荷）。
    /// `level`：上级走势级别（次级别 level + 1，Lean `Move.level` 严格递增/descend 递减）。
    /// 坐标取窗口首单元起点 + 末单元终点（上级走势覆盖其全部次级别走势的 K 序跨度）。
    ///
    /// `rmove` 的 `RMove::Compose.subs` = 窗口内次级别走势的 **rmove**（纯结构，坐标剥离——Lean μF
    /// 的 subs）；`sub_moves` 保留**携坐标**的 `subs`（坐标侧车，`descend_leveled` 取回 + `index_of_in`
    /// 映射 source_index）。两者同序同长（`sub_moves[i].rmove == rmove.subs[i]`）。
    pub fn compose(subs: &[LeveledMove], center: Center, level: u32) -> LeveledMove {
        let sub_rmoves: Vec<RMove> = subs.iter().map(|m| m.rmove.clone()).collect();
        let start_index = subs.first().map(|m| m.start_index).unwrap_or(0);
        let end_index = subs.last().map(|m| m.end_index).unwrap_or(0);
        LeveledMove {
            rmove: RMove::Compose {
                subs: sub_rmoves,
                centers: vec![center],
                level,
            },
            start_index,
            end_index,
            sub_moves: subs.to_vec(),
        }
    }

    /// 该走势的方向投影（L0 线段直接取方向；上级走势取外缘趋势——次级别坐标侧车折叠为
    /// `UnitRange` 时用，与旧塔 `classify_relation` 外缘判据同源）。
    ///
    /// ★诚实有效域：上级走势的方向是**外缘占位**（首单元下移/上移），**不**冒充 §6.1 意义的
    /// 线段方向交替（上级中枢检测用几何路径 `center_from_window`，不读方向——见 center.rs）。
    pub fn fold_direction(&self, prev: Option<&LeveledMove>) -> Direction {
        match prev {
            // 首单元无前驱 ⟹ 缺省 Up（几何路径不读取，结构占位）。
            None => Direction::Up,
            // 外缘上移（hi 升）= Up，下移 = Down（与 classify_relation 外缘判据同源）。
            Some(p) => {
                if self.rmove.hi() >= p.rmove.hi() {
                    Direction::Up
                } else {
                    Direction::Down
                }
            }
        }
    }
}

/// 从携坐标的次级别走势序列识别中枢序列 + **每个中枢的构成窗口**（窗口化 compose 的核心）。
///
/// 三段窗口扫描（成立支消费 3 段、不成立支消费 1 段，对齐 `Origin.centersOf` 滑窗终止性）。
/// 与 `mod.rs::detect_centers_with` 的扫描骨架**同构**，但额外返回每个中枢的构成三段索引
/// `(i, i+1, i+2)`——这是窗口封装为上级走势所需的 subs 来源（旧 `detect_centers_with` 只返回
/// 中枢，丢弃了构成窗口 ⟹ 无法 compose）。
///
/// `build`：中枢构造函数（L0=完整判据 `center_from_segments`；上级=几何 `center_from_window`）。
/// 返回 `Vec<(Center, [usize; 3])>`：每个中枢 + 构成它的三段次级别走势在 `units` 中的索引。
fn detect_centers_windowed(
    units: &[UnitRange],
    build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center>,
) -> Vec<(Center, [usize; 3])> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 2 < units.len() {
        match build(&units[i], &units[i + 1], &units[i + 2]) {
            Some(c) => {
                out.push((c, [i, i + 1, i + 2]));
                // 成立支：前进 3 段（已确认中枢不回写，reference:16）。
                i += 3;
            }
            None => {
                // 不成立支：前进 1 段继续找（对齐 Origin centersOf 滑窗）。
                i += 1;
            }
        }
    }
    out
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
    // 每个中枢的构成窗口（三段次级别 LeveledMove）→ compose 为一个上级走势。
    let upper: Vec<LeveledMove> = windowed
        .iter()
        .map(|(c, win)| {
            let subs = [
                subs_moves[win[0]].clone(),
                subs_moves[win[1]].clone(),
                subs_moves[win[2]].clone(),
            ];
            LeveledMove::compose(&subs, *c, level)
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
// ## 增量正确性（L0 纯结构证明，no-patch：非套用 zhongshu ScanResume 语义）
//
// `detect_centers_windowed` 是**确定性左折叠**：游标 `i` 从 0 严格递增（成立支 +3、不成立支
// +1），每步决策 `build(units[i],units[i+1],units[i+2])` 是纯函数（不依赖历史）。三条推论：
//
// 1. **路径确定性**：给定 `units[0..k]`，扫描到达位置 `k` 时的游标路径与已产出 centers 序列
//    完全确定（前缀的确定性函数）。
// 2. **已产出 centers 是不可变前缀**：成立支产出 center 后 `i+=3`，该 center 窗口 `[i,i+1,i+2]`
//    永不被后续重访（i 严格递增 ⟹ 窗口不重叠）。故已产出 centers 序列可缓存，续扫只追加尾部。
// 3. **续扫等价于全量重扫到达断点后继续**：从 `consumed` 续扫 == 全量重扫到达 `consumed`（前缀
//    路径不变）然后继续扫尾部新 units。
//
// `consumed`（退出断点）= while 退出时的游标 `i`（满足 `i+2 >= len`）。尾部追加后
// `consumed+2 < new_len` 可能成立 ⟹ 从 `consumed` 续扫的新窗口 `[consumed,consumed+1,consumed+2]`
// 可能横跨旧/新段——全量重扫也会到达同一 `consumed` 后扫同一窗口（前缀不变 ⟹ 同路径）。
//
// **与 zhongshu `ScanResume` 的严格区分**（no-patch：不套用不同语义）：zhongshu 处理中枢
// **延伸吸收**（unsettled 中枢携 extend 状态 gg/dd 吸收后续段），其 `Unsettled` 状态机在塔的
// 非重叠三段窗口扫描中**不存在**（塔成立支 +3 永不回头、不 extend）。塔增量基元是无状态的
// 游标续进（仅 `consumed` + 已产出不可变前缀），状态机更简单，直接基于左折叠的确定性。
//
// ## 真 Fugue 547（铁律保留）
//
// 增量 compose 的 `LeveledMove::compose` 父子仍用真 sub_moves（窗口三段次级别 LeveledMove），
// `descend` 取回真 subs ⟹ B2/S2 真可产。增量只改"扫描从何处起"，不改"compose 的 subs 来源"——
// subs 永远是真窗口三段（禁级别差伪造）。
//
// ## 认识论等级（formalization-validity-domain 231号）
//
// 增量等价性本身是 **L0**（纯结构，确定性左折叠的数学性质，不依赖数据）。bit-exact 逐 bar
// 断言是 **L1**（合成数据 + 真实数据管线正确性验证）。标度 exp≈1 是 **L2**（真实数据经验标度）。

/// 扫描退出断点（增量续进的锚）：while 退出时的游标位置。
///
/// `consumed` 满足 `consumed + 2 >= units.len()`（while 终止条件）。尾部追加 units 后，
/// `consumed + 2 < new_len` 可能成立 ⟹ 从 `consumed` 续扫正确（见模块文档增量证明）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WindowScanCursor {
    /// 已扫描到的游标位置（退出点；`units[..consumed]` 的扫描路径已确定）。
    pub consumed: usize,
}

/// 增量窗口扫描：从 `start_i` 续扫三段窗口，返回新产出的 `(Center, [usize;3])` 序列 + 退出断点。
///
/// 与 `detect_centers_windowed(units, build)` 的关系：
/// - 全量等价：`detect_centers_windowed(units, build)` == `detect_centers_windowed_resume(units, build, 0).0`
///   （`start_i=0` 续扫 == 全量扫描）。
/// - 增量等价：设上次扫描在 `units[..old_len]` 上退出断点为 `c0`（`c0.consumed`），产出前缀
///   `prefix`。追加到 `new_len` 后，`detect_centers_windowed_resume(units, build, c0.consumed)` 返回
///   `(tail, c1)`，则全量扫描 `detect_centers_windowed(units, build)` == `prefix ++ tail`（bit-exact）。
///
/// **bit-exact 充要条件**（调用方必须保证，否则增量破裂）：
/// 1. `start_i` 必须是前缀 `units[..start_i]` 的真实退出断点（上次扫描返回的 `consumed`）。
/// 2. `units[..start_i]` 在两次扫描间**不可变**（只允许尾部追加）。
/// 3. 已产出的前缀 centers 不可变（成立支 +3 ⟹ 永不重访，自动满足）。
///
/// 三个条件满足时，从 `start_i` 续扫产出的 tail 与全量重扫到达 `start_i` 后继续的产出逐位相同。
pub fn detect_centers_windowed_resume(
    units: &[UnitRange],
    build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center>,
    start_i: usize,
) -> (Vec<(Center, [usize; 3])>, WindowScanCursor) {
    let mut out = Vec::new();
    let mut i = start_i;
    while i + 2 < units.len() {
        match build(&units[i], &units[i + 1], &units[i + 2]) {
            Some(c) => {
                out.push((c, [i, i + 1, i + 2]));
                i += 3;
            }
            None => {
                i += 1;
            }
        }
    }
    (out, WindowScanCursor { consumed: i })
}

/// 增量 compose：从 `start_i` 续扫窗口 + 把新产出的窗口 compose 为上级 `LeveledMove`。
///
/// 与 `compose_level` 的关系：
/// - 全量等价：`compose_level(units, subs, is_l0, level)` ==
///   `compose_level_resume(units, subs, is_l0, level, 0)`（`.0`/`.1`/`.2` 三元组相同）。
/// - 增量：返回 `(tail_centers, tail_upper, cursor)`——tail 是新产出（追加到已缓存前缀后），
///   `cursor.consumed` 是退出断点（下次续扫起点）。
///
/// `subs_moves` 必须与 `units` 同序同长（`units` 是 `subs_moves` 的投影）。增量只追加产出，
/// **不修改**已缓存的 `LeveledMove` 前缀——真 Fugue 547：每个新 compose 的 subs 仍是真窗口三段
/// 次级别 LeveledMove（`subs_moves[win[0..3]]`），descend 取回真 subs。
pub fn compose_level_resume(
    units: &[UnitRange],
    subs_moves: &[LeveledMove],
    is_l0: bool,
    level: u32,
    start_i: usize,
) -> (Vec<Center>, Vec<LeveledMove>, WindowScanCursor) {
    let build = if is_l0 {
        super::center::center_from_segments
    } else {
        super::center::center_from_window
    };
    let (windowed, cursor) = detect_centers_windowed_resume(units, build, start_i);
    let tail_centers: Vec<Center> = windowed.iter().map(|(c, _)| *c).collect();
    let tail_upper: Vec<LeveledMove> = windowed
        .iter()
        .map(|(c, win)| {
            let subs = [
                subs_moves[win[0]].clone(),
                subs_moves[win[1]].clone(),
                subs_moves[win[2]].clone(),
            ];
            LeveledMove::compose(&subs, *c, level)
        })
        .collect();
    (tail_centers, tail_upper, cursor)
}

/// 把上级 `LeveledMove` 序列投影为 `UnitRange` 序列（供下一级 `detect_centers_windowed` 的
/// 几何路径用——上级中枢检测在外缘区间上做，方向是外缘占位）。
///
/// 上级走势的 `UnitRange` = `[rmove.lo, rmove.hi]`（外缘下沿/上沿，由 subs 区间聚合，descend.rs
/// `RMove::lo/hi`）+ 坐标 + 外缘趋势方向。**不**读取方向交替（几何路径 center_from_window 用，
/// 见 center.rs 诚实有效域）——方向是结构占位使 UnitRange 类型完整。
pub fn project_to_units(moves: &[LeveledMove]) -> Vec<UnitRange> {
    moves
        .iter()
        .enumerate()
        .map(|(idx, m)| {
            let prev = if idx == 0 { None } else { Some(&moves[idx - 1]) };
            UnitRange {
                start_index: m.start_index,
                end_index: m.end_index,
                direction: m.fold_direction(prev),
                lo: m.rmove.lo(),
                hi: m.rmove.hi(),
            }
        })
        .collect()
}

/// 携坐标下钻（`descend` 的坐标层镜像）：取回构成 `parent` 的**携坐标**次级别走势序列。
///
/// `descend`（descend.rs）取回的是裸 `RMove`（坐标剥离）——无法反查 source_index。本函数取回
/// `parent.sub_moves`（携坐标侧车，与 `descend(parent.rmove)` 同序同长，`sub_moves[i].rmove ==
/// descend(parent.rmove)[i]` 不变量）。L0 线段（递归底，`sub_moves` 空）⟹ 空序列（与 descend 一致）。
pub fn descend_leveled(parent: &LeveledMove) -> Vec<LeveledMove> {
    parent.sub_moves.clone()
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
    let lo = close_src.iter().position(|&s| s >= start)?;
    let hi = close_src.iter().rposition(|&s| s <= end)?;
    if lo > hi {
        return None;
    }
    Some((lo, hi))
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

    /// L0 线段单元 → RMove::Segment（递归底，坐标保留）。
    #[test]
    fn from_unit_is_segment_with_coords() {
        let u = unit(4, 8, down(), 90, 150);
        let lm = LeveledMove::from_unit(&u);
        assert_eq!(lm.rmove, RMove::Segment { direction: down(), lo: 90, hi: 150 });
        assert_eq!((lm.start_index, lm.end_index), (4, 8));
        // 递归底：descend 得空（L0 线段无次级别）。
        assert!(descend(&lm.rmove).is_empty());
    }

    /// ★组装-取回对偶（descend ∘ compose = id）：compose 三段次级别 → descend 取回它们的 rmove。
    #[test]
    fn compose_descend_roundtrip_preserves_subs() {
        let s0 = LeveledMove::from_unit(&unit(0, 4, up(), 0, 10));
        let s1 = LeveledMove::from_unit(&unit(4, 8, down(), 3, 12));
        let s2 = LeveledMove::from_unit(&unit(8, 12, up(), 5, 15));
        let c = Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: 0, end_index: 12 };
        let parent = LeveledMove::compose(&[s0.clone(), s1.clone(), s2.clone()], c, 1);
        // descend 取回三段次级别 rmove（旧塔 UnitRange 折叠后 descend 得空）。
        let subs = descend(&parent.rmove);
        assert_eq!(subs, vec![s0.rmove.clone(), s1.rmove.clone(), s2.rmove.clone()]);
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
        assert_eq!(*win, [0, 1, 2]); // 构成窗口三段索引
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
        let moves: Vec<LeveledMove> = units.iter().map(LeveledMove::from_unit).collect();
        let (centers, upper) = compose_level(&units, &moves, true, 1);
        assert_eq!(centers.len(), 1, "上-下-上 全三段核心非空 ⟹ 一个中枢");
        assert_eq!(upper.len(), 1, "一个中枢 ⟹ 一个上级走势");
        // 上级走势是 RMove::Compose，descend 取回构成它的三段 L0 线段。
        let subs = descend(&upper[0].rmove);
        assert_eq!(subs.len(), 3, "上级走势 descend 取回三段次级别走势（B2 可产的前提）");
        assert_eq!(subs, vec![moves[0].rmove.clone(), moves[1].rmove.clone(), moves[2].rmove.clone()]);
    }

    /// index_of_in：从坐标侧车按结构身份查回次级别走势的 end_index。
    #[test]
    fn index_of_maps_rmove_to_source_index() {
        let s0 = LeveledMove::from_unit(&unit(0, 4, up(), 0, 10));
        let s1 = LeveledMove::from_unit(&unit(4, 8, down(), 3, 12));
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
        let s0 = LeveledMove::from_unit(&unit(0, 4, up(), 0, 10));
        let s1 = LeveledMove::from_unit(&unit(4, 8, down(), 3, 12));
        let s2 = LeveledMove::from_unit(&unit(8, 12, up(), 5, 15));
        let c = Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: 0, end_index: 12 };
        let parent = LeveledMove::compose(&[s0, s1, s2], c, 1);
        let units = project_to_units(&[parent.clone()]);
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
        let (res, cursor) =
            detect_centers_windowed_resume(&units, super::super::center::center_from_segments, 0);
        assert_eq!(res.len(), full.len(), "start_i=0 续扫产出 == 全量");
        for (r, f) in res.iter().zip(full.iter()) {
            assert_eq!(r.0, f.0, "中枢相等");
            assert_eq!(r.1, f.1, "窗口索引相等");
        }
        // 9 段 → 3 窗口（成立支 +3 三次）⟹ consumed = 9（9+2 >= 9 退出）。
        assert_eq!(cursor.consumed, 9, "9 段全消费，退出断点 consumed=9");
    }

    /// ★增量核心 bit-exact：尾部追加后续扫 == 全量重扫。
    ///
    /// 前 6 段（2 窗口）扫描断点缓存，追加 3 段后从断点续扫 ⟹ 产出 == 全量 9 段扫描。
    /// 验证「已产出前缀不可变 + 从 consumed 续扫 == 全量」的增量不变量。
    #[test]
    fn resume_after_append_matches_full_rescan() {
        // 9 段交替（3 窗口），分两批：前 6 段 → 追加 3 段。
        let all_units: Vec<UnitRange> = (0..9)
            .map(|i| {
                let dir = if i % 2 == 0 { up() } else { down() };
                unit(i * 4, i * 4 + 4, dir, 0, 100)
            })
            .collect();
        let build = super::super::center::center_from_segments;

        // 全量基准。
        let full = detect_centers_windowed(&all_units, build);

        // 增量：前 6 段先扫。
        let (prefix, cursor6) = detect_centers_windowed_resume(&all_units[..6], build, 0);
        // 追加到 9 段后从 cursor6 续扫（前缀 units[..6] 不变，仅尾部追加）。
        let (tail, _cursor9) = detect_centers_windowed_resume(&all_units, build, cursor6.consumed);

        // 拼接 == 全量。
        let mut combined = prefix.clone();
        combined.extend(tail);
        assert_eq!(combined.len(), full.len(), "增量拼接长度 == 全量");
        for (c, f) in combined.iter().zip(full.iter()) {
            assert_eq!(c.0, f.0, "中枢相等");
            assert_eq!(c.1, f.1, "窗口索引相等");
        }
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
        let moves: Vec<LeveledMove> = units.iter().map(LeveledMove::from_unit).collect();

        // 全量 compose_level。
        let (full_c, full_u) = compose_level(&units, &moves, true, 1);

        // resume from 0 == 全量。
        let (rc, ru, _) = compose_level_resume(&units, &moves, true, 1, 0);
        assert_eq!(rc, full_c, "resume(0) centers == 全量");
        assert_eq!(ru, full_u, "resume(0) upper == 全量");

        // 增量：前 6 段 compose。
        let (pc, pu, cursor6) = compose_level_resume(&units[..6], &moves[..6], true, 1, 0);
        // 追加续扫。
        let (tc, tu, _) = compose_level_resume(&units, &moves, true, 1, cursor6.consumed);

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
        let (prefix, c0) = detect_centers_windowed_resume(&units[..2], build, 0);
        assert!(prefix.is_empty(), "2 段凑不齐窗口 ⟹ 空产出");
        assert_eq!(c0.consumed, 0, "len=2 不进 while ⟹ consumed=0");
        // 追加到 5 段从 consumed=0 续扫 == 全量。
        let (tail, _) = detect_centers_windowed_resume(&units, build, c0.consumed);
        assert_eq!(tail.len(), full.len(), "从 0 续扫 == 全量");
        for (t, f) in tail.iter().zip(full.iter()) {
            assert_eq!(t.0, f.0);
            assert_eq!(t.1, f.1);
        }
    }

    /// 多级递归塔：L0 → L1 → L2，每级 descend 取回下级走势（级别严格递减）。
    #[test]
    fn multi_level_tower_descend_decreases_level() {
        // 9 段 L0 全重叠 [0,100]（交替方向），逐级 compose。
        let units: Vec<UnitRange> = (0..9)
            .map(|i| {
                let dir = if i % 2 == 0 { up() } else { down() };
                unit(i * 4, i * 4 + 4, dir, 0, 100)
            })
            .collect();
        let moves: Vec<LeveledMove> = units.iter().map(LeveledMove::from_unit).collect();
        // L0 → L1（完整判据，方向交替）。
        let (_c1, l1) = compose_level(&units, &moves, true, 1);
        assert_eq!(l1.len(), 3, "9 段 → 3 窗口 → 3 个 L1 走势");
        for m in &l1 {
            assert_eq!(m.rmove.level(), 1);
            // L1 走势 descend 取回 3 段 L0 线段（level 0）。
            for sub in descend(&m.rmove) {
                assert_eq!(sub.level(), 0, "L1 descend 得 L0 线段");
            }
        }
        // L1 → L2（几何路径）。
        let l1_units = project_to_units(&l1);
        let (_c2, l2) = compose_level(&l1_units, &l1, false, 2);
        assert_eq!(l2.len(), 1, "3 个 L1 走势 → 1 窗口 → 1 个 L2 走势");
        assert_eq!(l2[0].rmove.level(), 2);
        // L2 走势 descend 取回 3 个 L1 走势（level 1）。
        for sub in descend(&l2[0].rmove) {
            assert_eq!(sub.level(), 1, "L2 descend 得 L1 走势");
        }
    }
}
