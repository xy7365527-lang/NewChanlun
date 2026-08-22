//! 身份键与桥（#1188 B01 职责块自 `nest_lifecycle.rs` 迁出，零行为）。

use super::*;

// ═══════════════════════════════════════════════════════════════════════════
// 身份键与桥（卡 §2.2/§6.3）
// ═══════════════════════════════════════════════════════════════════════════

/// 生命周期身份键（E2E §6.1:248：状态、钟与行进中的区间不进入事件身份键）。
///
/// 六元组 = `(level, side, kind, seg_a, seg_c_full, b_center_start)`。trend 域
/// `seg_c_full` 取事件产出坐标 `interval_b`（工程桥，模块头 090 登记 1）；pan 活窗域
/// 取 `(c_start_live, live_end)`（卡 §3）。`b_center_start` = B 中枢身份快照
/// （level_view.rs:495-503 prefix 首次观察快照纪律，延伸不改写 start_index）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleKey {
    pub level: u32,
    pub side: Side,
    pub kind: NestDivergenceKind,
    pub seg_a: (usize, usize),
    pub seg_c_full: (usize, usize),
    pub b_center_start: usize,
}

/// `Side` 无 Ord derive（types.rs 口径不动）——排序/判等经判别值投影，不改上游类型。
pub(super) fn side_tag(side: Side) -> u8 {
    match side {
        Side::Long => 0,
        Side::Short => 1,
    }
}

impl LifecycleKey {
    fn sort_tuple(
        &self,
    ) -> (
        u32,
        u8,
        NestDivergenceKind,
        (usize, usize),
        (usize, usize),
        usize,
    ) {
        (
            self.level,
            side_tag(self.side),
            self.kind,
            self.seg_a,
            self.seg_c_full,
            self.b_center_start,
        )
    }
}

impl PartialOrd for LifecycleKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LifecycleKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_tuple().cmp(&other.sort_tuple())
    }
}

/// 白名单工程桥（卡 §6.3 原样）：除 seg_c 右端外全等判同身份。
///
/// 吸收三形态——trend 确认收束（`[c_start, c_end] → [c_start, t*]`）、T5-OR 终假回扩、
/// pan 活窗延展（右端随 as_of 前进，设计内行为）。seg_a 改变 / seg_c 左端改变 / 其他任何
/// 分量改变 ⟹ 越界不判同身份（走身份消失路径，T8 负面对照锁定）。
///
/// 口径登记（spec #232 ID-5 字面张力）：ID-5「新 interval_b ⊆ 旧」仅覆盖收束一形态；
/// spec Solution 三形态（收束/回扩/活窗延展）与 T1（回扩）/T3（延展）锚定要求右端双向
/// 可动——本函数取三形态侧（右端不限方向），字面张力归编排者澄清（实装说明 20260724
/// 对照节登记，本票不替裁）。
pub(super) fn bridge_identity(a: &LifecycleKey, b: &LifecycleKey) -> bool {
    a.level == b.level
        && a.side == b.side
        && a.kind == b.kind
        && a.seg_a == b.seg_a
        && a.seg_c_full.0 == b.seg_c_full.0
        && a.b_center_start == b.b_center_start
}

/// 同**锚**判定（票 #559）：身份键去掉 `seg_c_full` 的五元相等。
///
/// 与 [`bridge_identity`] 的分工：桥要求 C 左端也相同（右端延展吸收为同身份）；同锚只要求
/// A/B 锚与级别/方向/域相同，**允许 C 左端不同**——那正是「provider 换到下一个 C」的形态。
/// 两者关系：`bridge_identity ⟹ same_anchor`（严格更强）。
pub(super) fn same_anchor(a: &LifecycleKey, b: &LifecycleKey) -> bool {
    a.level == b.level
        && a.side == b.side
        && a.kind == b.kind
        && a.seg_a == b.seg_a
        && a.b_center_start == b.b_center_start
}

/// 中枢升级认领判据（票 #603 档 1「暂认中枢回溯认领」，#599 编排者裁定 2026-07-28）。
///
/// **严格同锚**：`level/side/kind/seg_a/seg_c_full.0` 五项全等（= 同一背驰假设的同一 C 段），
/// **只放开 `b_center_start`**，且要求新 B 严格晚于旧 B（`old < new`，单调前进）。
///
/// 语义（为什么这不是"回填历史"）：B 的取值来自
/// [`nearest_confirmed_center_idx`] 对**已确认**中枢集合的查询——中枢从"未确认"变为"已确认"
/// 是回顾性、单调的状态跃迁，查询答案随之更新反映的是「当下可得的已确认信息集合变大了」，
/// 不是对未来的预判（`parser/tail.rs:11-14` 裁定的边界在此侧）。与 [`bridge_identity`] 已经
/// 承认的「C 收束/回扩/延展」是同一原则的推广：同一 A/C 组合，用当下最新已确认信息重算一个
/// 辅助参数。**单调方向是合法性的前提**——反向替换（用更早的中枢覆盖当前答案）才真正构成
/// 回填，本判据禁之。
///
/// **跨锚零实装**（#599 §4.2 教义否定，编排者采纳）：`seg_a` 是背驰检验的离开段，两个不同
/// `seg_a` 是两个**不同的背驰假设**（巧合共享同一 C 段），合并 = 用一个假设的最终结果回溯
/// 重定义另一个假设曾经的观测内容 = `tail.rs`「强行分类为最终结果」。故 `seg_a` 全等是硬
/// 约束，本函数不提供任何放开它的分支。
///
/// 与 [`bridge_identity`] 的关系：两者**互斥**（桥要求 `b_center_start` 相等，本判据要求严格
/// 不等）⟹ `advance` 第 1 步先桥后认领，匹配不重叠。C 右端不进本判据（与桥同款，右端随
/// as_of/收束变动，不是身份）。
pub(super) fn bridge_by_center_upgrade(old: &LifecycleKey, new: &LifecycleKey) -> bool {
    old.level == new.level
        && old.side == new.side
        && old.kind == new.kind
        && old.seg_a == new.seg_a
        && old.seg_c_full.0 == new.seg_c_full.0
        && old.b_center_start < new.b_center_start
}
