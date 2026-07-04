//! 走势类型分解（PDF §6/§9.2 + Q1/Q8 裁决，task #143）。
//!
//! canonical 中枢关系链 R_i = classify_relation(C_i, C_{i+1}) 切成 **maximal 等标签 run**：
//! Up/Down run → 趋势块（≥2 中枢自动满足「≥2 依次同向中枢」），LevelExpansion run → 盘整块
//! （中心定理二升级候选，#145 承接）。C_ℓ = B₁⊕…⊕B_k 完全分解，替代 AllTrend（全历史累积链
//! 全链同向）——后者是吸收锁死谓词（PDF §2：一对异向关系出现后 τ 永为退化，外审漏斗坐实
//! BTC 全历史在数据第 1-17 天锁死）。
//!
//! ## 完备性（span/ownership 双投影，设计 decomp-design-20260703.md §2）
//!
//! - **span**：块 j 覆盖中枢 [start..end]（闭区间），相邻块共享恰一个边界中枢（转折中枢，
//!   「走势终完美」——它完成前一走势类型，同时是后一类型的关系起点）。
//! - **ownership**（严格分区）：中枢 C_i 归包含关系 R_{i-1} 的块（转折中枢归前块），C_0 归 B₁。
//!   分区性质 ⟺ span 连续性不变量：`b[j+1].start == b[j].end` ∧ 每关系恰属一块（测试验证）。
//!
//! ## 前缀稳定性（批式 = 因果）
//!
//! fold 从左到右贪心，sealed 关系（两端中枢均不再变）折出的块边界不因追加中枢改变 ⟹ 全链
//! 分解的块结构 = 各时点「当前走势类型」的历史序列。段的最近已确认中枢 c 在其当时的当前块中
//! 恒为尾中枢——这使 `center_trend_gate` 的逐段批式门 = Q8 因果语义（「最后一个中枢」相对
//! 当前走势类型，非全历史最近）。
//!
//! ## 增量（T^inc == T^full 定义性成立）
//!
//! 单一来源 [`decompose_resume`]：全量 = 空 state 的 resume。冻结不变量：#142 resume 协议只
//! pop/改写**末位**中枢（前缀 sealed）⟹ 关系 R_i 冻结 ⟺ i < m-2（两端中枢均有后继）。临时
//! 尾关系 R_{m-2}（触 frontier 中枢，延伸可翻转其标签）每次调用重折，不入冻结前缀。

use super::super::types::{Center, Direction, MoveKind};
use super::center::{classify_relation, CenterRelation};

/// 块状态（Q8 CurrentMove 五元组之 status）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveStatus {
    /// 当前走势类型（链尾块——尾关系/尾中枢仍可变）。
    Active,
    /// 已完成（后继块已开启，前缀不可变）。
    Completed,
}

/// 走势类型块（PDF §9.2 M=(C_a..C_b,kind,dir,status)）。链尾 Active 块即 Q8 CurrentMove。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveBlock {
    /// span 首中枢下标（含；与前块共享——转折中枢 ownership 归前块）。
    pub start_center: usize,
    /// span 尾中枢下标（含）。Trend 块的「最后一个中枢」= centers[end_center]（Q8）。
    pub end_center: usize,
    pub kind: MoveKind,
    /// Trend 携方向；Consolidation = None（第31课盘整无方向）。
    pub dir: Option<Direction>,
    pub status: MoveStatus,
}

/// 折入一条关系（i = 关系下标，连接 C_i 与 C_{i+1}）。同标签且连续 ⟹ 延续尾块（maximal）；
/// 否则开新块（span 首 = i，与前块尾共享边界中枢）。
fn fold_rel(blocks: &mut Vec<MoveBlock>, rel: CenterRelation, i: usize) {
    let (kind, dir) = match rel {
        CenterRelation::UpContinuation => (MoveKind::Trend, Some(Direction::Up)),
        CenterRelation::DownContinuation => (MoveKind::Trend, Some(Direction::Down)),
        CenterRelation::LevelExpansion => (MoveKind::Consolidation, None),
    };
    match blocks.last_mut() {
        Some(last) if last.kind == kind && last.dir == dir && last.end_center == i => {
            last.end_center = i + 1;
        }
        _ => blocks.push(MoveBlock {
            start_center: i,
            end_center: i + 1,
            kind,
            dir,
            status: MoveStatus::Completed,
        }),
    }
}

/// 增量分解状态（进 LevelCache，与 cached_bsp 同回退纪律：cascade_reset/clear ⟹ [`reset`]）。
///
/// [`reset`]: DecomposeState::reset
#[derive(Debug, Clone, Default)]
pub struct DecomposeState {
    /// 冻结关系折出的块前缀（不含临时尾关系；status 在输出时统一设定）。
    frozen: Vec<MoveBlock>,
    /// 已冻结关系数（= 下一条待冻结关系的下标）。
    frozen_rels: usize,
}

impl DecomposeState {
    /// 退化为下次全量重折（centers 前缀改写/回缩时调用，同 LevelCache 其余缓存纪律）。
    pub fn reset(&mut self) {
        self.frozen.clear();
        self.frozen_rels = 0;
    }
}

/// 分解（resume 单一来源；全量 = 空 state）。返回 C_ℓ = B₁⊕…⊕B_k，链尾块 Active 其余 Completed。
///
/// `frontier_centers`：链尾可变中枢数。#142 旧协议恒 1（只有末位中枢开放）；#148 升级重切后
/// 末窗口可产 k 个子中枢且在窗口 sealed 前**全部**可变（窗口段数增长会改写末子中枢外缘、甚至
/// 改变子中枢数量），调用方传该窗口产出数（`WindowScanCursor::last_window_emitted`）。冻结边界
/// 随之后移：R_i 冻结 ⟺ 右端中枢 C_{i+1} 在可变尾之前。保守取大值恒正确（少冻结 = 多重折，
/// 输出不变——输出恒等于全折叠，fr 只影响缓存量）。
///
/// 冻结推进 O(新 sealed 关系数)，临时尾重折 O(frontier)，输出 clone O(k)（k = 块数 ≤ 关系标签
/// 变化数+1，L0 全历史量级为百级）。
pub fn decompose_resume(
    centers: &[Center],
    state: &mut DecomposeState,
    frontier_centers: usize,
) -> Vec<MoveBlock> {
    let m = centers.len();
    let n_rels = m.saturating_sub(1);
    // 关系 R_i 冻结 ⟺ i+1 < m-fr（右端中枢有后继且不在可变尾）。fr=1 时退化为旧 #142 边界 m-2。
    let fr = frontier_centers.max(1);
    let frozen_target = m.saturating_sub(1 + fr);
    if state.frozen_rels > frozen_target {
        state.reset(); // 回缩 ⟹ 全量重折（cascade_reset 已显式 reset，此为兜底护栏）。
    }
    while state.frozen_rels < frozen_target {
        let i = state.frozen_rels;
        fold_rel(&mut state.frozen, classify_relation(&centers[i], &centers[i + 1]), i);
        state.frozen_rels += 1;
    }
    let mut out = state.frozen.clone();
    // 临时尾关系（恰 R_{m-2}，触 frontier 中枢——延伸改写可翻转标签，故每次重折不入冻结）。
    for i in state.frozen_rels..n_rels {
        fold_rel(&mut out, classify_relation(&centers[i], &centers[i + 1]), i);
    }
    if out.is_empty() && m == 1 {
        // PDF §6 盘整块情形1：单中枢无同向后继（关系出现后由 fold 路径接管，无中链升格残留）。
        out.push(MoveBlock {
            start_center: 0,
            end_center: 0,
            kind: MoveKind::Consolidation,
            dir: None,
            status: MoveStatus::Completed,
        });
    }
    if let Some(last) = out.last_mut() {
        last.status = MoveStatus::Active;
    }
    out
}

/// 全量分解（= 空 state 的 resume，定义性等价；fr 任意合法值输出相同，取 1）。
pub fn decompose(centers: &[Center]) -> Vec<MoveBlock> {
    decompose_resume(centers, &mut DecomposeState::default(), 1)
}

/// 当前/链尾趋势块（PDF §9.3 取块口径；「刚完成趋势块」的可用时限窗口归 #144 判据域）。
pub fn last_trend_block(blocks: &[MoveBlock]) -> Option<&MoveBlock> {
    blocks.last().filter(|b| b.kind == MoveKind::Trend)
}

/// 每中枢局部趋势门（Q1/Q8，signal.rs 一类 τ 门消费）。
///
/// `gate[i] = Some(d)` ⟺ 中枢 i 落在某 Trend(d) 块 span 内且 i > 块首——即关系 R(i-1,i) 属该块
/// ⟹ 当中枢 i 是最新中枢时，当前走势类型 = 该趋势块、i = 其「最后一个中枢」、i-1 = 同块前驱
/// （A 段所在，PDF §7 C_prev）。前缀稳定性使该批式查表 = 因果判定（模块头）。
pub fn center_trend_gate(n_centers: usize, blocks: &[MoveBlock]) -> Vec<Option<Direction>> {
    let mut gate = vec![None; n_centers];
    for b in blocks {
        if b.kind == MoveKind::Trend {
            for g in &mut gate[b.start_center + 1..=b.end_center] {
                *g = b.dir;
            }
        }
    }
    gate
}

/// 单中枢 ownership 块方向查询（Q7 高级别方向，task #145——[`center_trend_gate`] 的逐点版，
/// `project_to_units{,_resume}` tail 投影消费，不 materialize 全 gate 向量——增量路径每 bar 只投影
/// 新 tail，全向量分配是 O(m·bars) 回归）。
///
/// 语义与 `center_trend_gate` 逐点相等（等价性测试锁定）：`Some(d)` ⟺ 关系 R(i-1,i) 属 Trend(d)
/// 块（ownership：C_i 归其入边关系的块）。`None` ⟺ i==0（C_0 无入边关系——ownership 的「C_0 归
/// B₁」需要 R(0,1) 即未来信息，因果投影层不可消费，降级 endpoint fallback；与 [`center_block_kind`]
/// 的 C_0 规则**有意不同**——kind 版是批式查询无前缀稳定约束）或 R(i-1,i) 属 Consolidation 块
/// （第31课盘整无方向 ⟹ 投影层降级 endpoint fallback）。
pub fn center_own_dir_at(blocks: &[MoveBlock], i: usize) -> Option<Direction> {
    if i == 0 {
        return None;
    }
    // 关系 R(i-1,i)（关系下标 i-1）属块 b ⟺ b.start_center ≤ i-1 < b.end_center ⟺ start < i ≤ end。
    // blocks 按 span 升序（fold 从左到右）⟹ partition_point 二分。
    let bi = blocks.partition_point(|b| b.end_center < i);
    blocks.get(bi).filter(|b| b.start_center < i).and_then(|b| b.dir)
}

/// 每中枢 ownership 块类别（Q4 盘整背驰承接路由，task #145——signal.rs 盘整块判定消费）。
///
/// `kind[i] = Some(k)` ⟺ 中枢 i 按 **ownership 分区**（模块头：C_i 归包含关系 R(i-1,i) 的块，
/// 转折中枢归前块，C_0 归 B₁）属于类别 k 的块。与 [`center_trend_gate`] 同一 `blocks` 单一来源
/// （不 fork 第二套分解）；区别：trend gate 只对 Trend 块开门且块首不开（关系起点无 A 段前驱），
/// 本函数给出**全部**中枢的块类别（含单中枢盘整块的 C_0——PDF §6 情形1，盘整=恰1中枢）。
pub fn center_block_kind(n_centers: usize, blocks: &[MoveBlock]) -> Vec<Option<MoveKind>> {
    let mut kinds = vec![None; n_centers];
    for b in blocks {
        // 关系 R(i-1,i) 属块 b ⟺ i ∈ (start_center, end_center]（ownership：转折中枢归前块）。
        for k in &mut kinds[(b.start_center + 1).min(n_centers)..(b.end_center + 1).min(n_centers)] {
            *k = Some(b.kind);
        }
    }
    // C_0 归 B₁（模块头 ownership 分区；单中枢链 blocks=[Consolidation 0..0] 由此覆盖）。
    if let (Some(first), Some(b0)) = (kinds.first_mut(), blocks.first()) {
        *first = Some(b0.kind);
    }
    kinds
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 外缘依次上移的中枢族（对齐 level.rs 旧测试见证）：k=0,1,2… 产 [4k, 4k+3] 外缘。
    fn c_up(k: i64) -> Center {
        Center { dd: 4 * k, zd: 4 * k + 1, zg: 4 * k + 2, gg: 4 * k + 3, start_index: 0, end_index: 0 }
    }
    /// 与 c_up(k) 外缘重叠的中枢（LevelExpansion 关系）。
    fn c_overlap(k: i64) -> Center {
        Center { dd: 4 * k + 1, zd: 4 * k + 2, zg: 4 * k + 2, gg: 4 * k + 4, start_index: 0, end_index: 0 }
    }

    fn kinds(blocks: &[MoveBlock]) -> Vec<(MoveKind, Option<Direction>, usize, usize)> {
        blocks.iter().map(|b| (b.kind, b.dir, b.start_center, b.end_center)).collect()
    }

    #[test]
    fn empty_and_single() {
        assert!(decompose(&[]).is_empty());
        let b = decompose(&[c_up(0)]);
        assert_eq!(kinds(&b), vec![(MoveKind::Consolidation, None, 0, 0)]);
        assert_eq!(b[0].status, MoveStatus::Active);
    }

    #[test]
    fn pure_up_chain_one_trend_block() {
        // 全链同向 = 分解恰 1 块（与旧 AllTrend 谓词唯一重合情形）。
        let cs = [c_up(0), c_up(1), c_up(2)];
        let b = decompose(&cs);
        assert_eq!(kinds(&b), vec![(MoveKind::Trend, Some(Direction::Up), 0, 2)]);
    }

    #[test]
    fn mixed_chain_no_lock_in() {
        // 不锁死前件（PDF §10）：早期 overlap 后接局部同向 run——旧 AllTrend 永退化，
        // 新分解产出尾部 Trend 块（#144 门可开）。
        let cs = [c_up(0), c_overlap(0), c_up(2), c_up(3), c_up(4)];
        let b = decompose(&cs);
        // R0=overlap（c_up(0) 与 c_overlap(0) 外缘交叠）、R1..R3 = Up。
        assert_eq!(
            kinds(&b),
            vec![
                (MoveKind::Consolidation, None, 0, 1),
                (MoveKind::Trend, Some(Direction::Up), 1, 4),
            ]
        );
        assert_eq!(last_trend_block(&b).unwrap().end_center, 4);
        assert_eq!(b[0].status, MoveStatus::Completed);
        assert_eq!(b[1].status, MoveStatus::Active);
        // 门：块首（关系起点中枢）不开门，块内后继开门。
        let gate = center_trend_gate(cs.len(), &b);
        assert_eq!(gate, vec![None, None, Some(Direction::Up), Some(Direction::Up), Some(Direction::Up)]);
    }

    #[test]
    fn reversal_shares_boundary_center() {
        // [Up, Down]：转折中枢 C1 为上涨块尾（ownership 归它）且下跌块 span 首（关系起点）。
        let cs = [c_up(0), c_up(2), c_up(1)];
        let b = decompose(&cs);
        assert_eq!(
            kinds(&b),
            vec![
                (MoveKind::Trend, Some(Direction::Up), 0, 1),
                (MoveKind::Trend, Some(Direction::Down), 1, 2),
            ]
        );
    }

    #[test]
    fn center_own_dir_at_equals_trend_gate_pointwise() {
        // Q7 等价性锁定：逐点查询版 == center_trend_gate 全向量（含块首/盘整块/C_0 的 None）。
        let chains: Vec<Vec<Center>> = vec![
            vec![c_up(0)],
            vec![c_up(0), c_up(1), c_up(2)],
            vec![c_up(0), c_overlap(0), c_up(2), c_up(3), c_up(4)],
            vec![c_up(5), c_up(3), c_overlap(3), c_up(6)], // down→expansion→up 混合链
        ];
        for cs in &chains {
            let blocks = decompose(cs);
            let gate = center_trend_gate(cs.len(), &blocks);
            for i in 0..cs.len() {
                assert_eq!(center_own_dir_at(&blocks, i), gate[i],
                    "逐点 ownership 方向须 == trend gate[{i}]（链长 {}）", cs.len());
            }
        }
    }

    #[test]
    fn center_block_kind_ownership_partition() {
        // 单中枢：blocks=[Consolidation 0..0] ⟹ C_0 归 B₁ = Consolidation。
        let single = decompose(&[c_up(0)]);
        assert_eq!(center_block_kind(1, &single), vec![Some(MoveKind::Consolidation)]);
        // 混合链 [Consolidation 0..1, Trend(Up) 1..4]：C_0 归 B₁（盘整）、C_1 转折中枢归前块
        // （盘整）、C_2..C_4 归趋势块——与 center_trend_gate 同 blocks 单一来源，ownership 分区。
        let cs = [c_up(0), c_overlap(0), c_up(2), c_up(3), c_up(4)];
        let b = decompose(&cs);
        assert_eq!(
            center_block_kind(cs.len(), &b),
            vec![
                Some(MoveKind::Consolidation),
                Some(MoveKind::Consolidation),
                Some(MoveKind::Trend),
                Some(MoveKind::Trend),
                Some(MoveKind::Trend),
            ]
        );
    }

    /// 完备性 property（设计 §6 测试1）：span 连续（分区 ⟺ b[j+1].start==b[j].end）、
    /// 首尾覆盖、maximal（相邻块异标签）、Trend 块 span≥2、关系计数恰 m-1。
    fn assert_complete(centers: &[Center], blocks: &[MoveBlock]) {
        let m = centers.len();
        if m <= 1 {
            return;
        }
        assert_eq!(blocks.first().unwrap().start_center, 0);
        assert_eq!(blocks.last().unwrap().end_center, m - 1);
        let mut rels = 0usize;
        for w in blocks.windows(2) {
            assert_eq!(w[1].start_center, w[0].end_center, "span 连续（ownership 分区）");
            assert!(
                (w[0].kind, w[0].dir) != (w[1].kind, w[1].dir),
                "maximality：相邻块必异标签"
            );
        }
        for b in blocks {
            assert!(b.end_center > b.start_center, "关系块 span ≥2 中枢");
            rels += b.end_center - b.start_center;
        }
        assert_eq!(rels, m - 1, "每关系恰属一块");
    }

    /// #148 升级重切协议：末窗口 pop 1 产 3（8→9 段重切）——链尾可变中枢数从 1 变 3，
    /// 调用方传 frontier_centers=窗口产出数 ⟹ 冻结前缀不含被改写关系，inc == full。
    #[test]
    fn multi_center_frontier_rewrite_parity() {
        let mut state = DecomposeState::default();
        // 阶段1：sealed 前缀 [c_up(0), c_up(1)] + 末窗口产 1 个延伸中枢。
        let mut centers = vec![c_up(0), c_up(1), c_overlap(1)];
        let inc = decompose_resume(&centers, &mut state, 1);
        assert_eq!(inc, decompose(&centers), "阶段1 inc == full");
        // 阶段2：窗口跨越升级阈值——pop 末 1 个，重扫产 3 个子中枢（值与数量均变）。
        centers.pop();
        centers.extend([c_overlap(1), c_up(3), c_overlap(3)]);
        let inc = decompose_resume(&centers, &mut state, 3);
        assert_eq!(inc, decompose(&centers), "阶段2（一窗多产改写）inc == full");
        // 阶段3：窗口继续增长——pop 末 3 个重产 4 个（末子中枢外缘改写 + 新子中枢）。
        for _ in 0..3 {
            centers.pop();
        }
        centers.extend([c_overlap(1), c_up(3), c_up(4), c_overlap(4)]);
        let inc = decompose_resume(&centers, &mut state, 4);
        assert_eq!(inc, decompose(&centers), "阶段3（可变尾加深）inc == full");
    }

    #[test]
    fn completeness_and_resume_parity_randomized() {
        // 无 proptest 依赖：LCG 自造随机链 + 增量事件流（追加/frontier 改写/回缩）。
        let mut seed: u64 = 0x5eed_cafe;
        let mut rng = move || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as i64
        };
        for _ in 0..200 {
            let mut centers: Vec<Center> = Vec::new();
            let mut state = DecomposeState::default();
            for step in 0..40 {
                let r = rng().rem_euclid(100);
                if r < 60 || centers.is_empty() {
                    // 追加：随机档位制造 Up/Down/Overlap 混合关系。
                    centers.push(c_up(rng().rem_euclid(8)));
                } else if r < 90 {
                    // frontier 改写（#142 延伸语义：末位中枢 pop+重扫改值）。
                    let last = centers.len() - 1;
                    centers[last] = c_up(rng().rem_euclid(8));
                } else {
                    // 回缩（cascade_reset 情形——生产路径会显式 reset，此处走兜底护栏）。
                    centers.pop();
                }
                let inc = decompose_resume(&centers, &mut state, 1);
                let full = decompose(&centers);
                assert_eq!(inc, full, "T^inc == T^full（step {step}）");
                assert_complete(&centers, &full);
                if let Some(last) = full.last() {
                    assert_eq!(last.status, MoveStatus::Active);
                    assert!(full[..full.len() - 1].iter().all(|b| b.status == MoveStatus::Completed));
                }
            }
        }
    }
}
