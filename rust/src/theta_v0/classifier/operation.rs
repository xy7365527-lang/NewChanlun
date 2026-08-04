//! 操作分解层（ADR 0011，#881 S3）：**横向旁路读法**——拿第 k 层元素用**不延伸**规则重折，
//! 产该级别操作序列（含并列盘整）。
//!
//! ## 工位定位（裁定一：一等模块，不是塔的投影）
//!
//! - **输入** = 塔第 k 层元素（`units`，级别循环内现场）+ seed 判据 `build`；
//! - **输出** = 第 k 层的不延伸分解序列 [`OperationSequence`]（中枢 + 走势类型块）；
//! - **不变式** = **分解唯一性**——`038-第38课.md:18`【正文】「一个好的分解，其分解规则下，必须
//!   保证分解的唯一性」；`038-第38课.md:28`【正文】「按照以上的同级别分解规则，用结合律很容易
//!   证明，这种分解下，其分解也是唯一的」（行号 2026-08-04 grep 本体核验）。
//!
//! 投影方案（方案 B）被 ADR 0011 否决：构造层的延伸合并是**有损**的（连续震荡被吸收成一个，
//! 摊不回去），从塔产出反推造不出被丢掉的信息。故旁路必须在级别循环**内**拿 units 现场重折——
//! 这是「接口变更而非查询函数」的落点。
//!
//! ## 不延伸规则（裁定六：规则按角色分，不按级别分）
//!
//! `038-第38课.md:22`【正文】「在这种同级别的分解中，是不需要中枢延伸或扩展的概念的……如果这
//! 5分钟次级别延伸出（延伸成）6段，那么就当成两个30分钟盘整类型的连接，在这种分解中，是允许
//! 盘整+盘整情况的」。规则差只有一处：**延伸的吸收步被摘掉**——
//!
//! - **seed 判据与主干同一**：`build` 由调用方按塔自己的级别分派传入（L0 `center_from_segments`／
//!   上级 `center_from_window`，#804 登记在案的唯一缝，Lean 背书），旁路**不另立判据**；
//! - **窗口恒 3 段、成立即跳 3**：`i += 3`，不做 `detect_centers_windowed_resume` Step2 的延伸
//!   吸收，也无 ≥9 段重切（#826 探针的「口径 S」直译转正）；
//! - **折叠不合并 LevelExpansion**：[`decompose::fold_rel`] 的 maximal 同标签连段合并属构造
//!   （ADR 0011 裁定四），旁路不继承——相邻重叠中枢各自独立成块，**盘整+盘整**由此可表示；
//! - **趋势块仍合并**：同向（Up/Down）关系 maximal run 折成一个趋势块——「趋势 = ≥2 依次同向
//!   中枢」是走势分解定理的定义性判据，**不是延伸**（延伸管的是中枢震荡的吸收，与趋势无关）。
//!
//! ## 块序列的铺排纪律（与 decompose 的 span 语义**有意不同**，勿混用查询假设）
//!
//! - 趋势块 `[start..end]` 覆盖关系 `start..end-1`（≥2 中枢）；相邻趋势块在转折点**共享**边界
//!   中枢（与 decompose 一致）；
//! - 盘整块 = **单中枢**块 `[c..c]`（038：一个中枢就是一个盘整走势类型）；相邻两个盘整块
//!   （`[i..i],[i+1..i+1]`）即「盘整+盘整」连接，**不共享**边界中枢；
//! - 块按 `(start_center, end_center)` 升序；`start_center`/`end_center` 索引的是
//!   **本序列自己的 `centers`**（口径 S 中枢链，与主干该级中枢链**不同一份**）；
//! - 链尾块 `Active` 其余 `Completed`（沿用 decompose 的状态纪律：尾块随新元素仍可变）。
//!
//! ## ★ 旁路结果不回流主干（裁定七）
//!
//! 本模块全是纯函数：只读 `units`，不写塔的任何状态。挂载点的「主干逐位不变」由
//! `mod.rs` 的集成测试锁（`classify` vs `classify_with_operations` 的 `Classification` 全等）
//! 承担，不靠本模块自觉。
//!
//! ## 现状与遗留
//!
//! - 本模块只接**全量** `classify` 族（`classify_with_operations` 入口）。增量塔（TowerCache）
//!   的 per-bar 旁路维护需要 resume 版口径 S 扫描（否则 O(n²)），属后续实施票，未做。
//! - 「避开下跌类型」的筛段开关**不存在**（ADR 0011 裁定五，多空双开）——操作序列照样报
//!   上涨/下跌/盘整事实，不提供筛段开关。

use super::super::types::{Center, Direction, MoveKind};
use super::center::{classify_relation, CenterRelation, UnitRange};
use super::decompose::{MoveBlock, MoveStatus};

/// 第 k 层操作序列（ADR 0011 裁定一输出物）：口径 S 中枢链 + 不延伸折叠的走势类型块。
///
/// `centers[i]` 由单元窗口 `windows[i] = (a, a+2)`（unit 索引闭区间，恒 3 段）经 seed 判据
/// 真派生；`blocks` 的 `start_center`/`end_center` 索引 **`centers`**（本序列私有链）。
#[derive(Debug, Clone, PartialEq)]
pub struct OperationSequence {
    /// 挂载的操作级别 k（由挂载点盖章）。
    pub level: u32,
    /// 口径 S 中枢链：恒 3 段窗口、禁延伸——与主干该级中枢链不是同一份（延伸被摘掉的产物）。
    pub centers: Vec<Center>,
    /// 与 `centers` 1:1 的单元窗口（unit 索引闭区间，恒 `(a, a+2)`）——provenance 供测试与
    /// 下游坐标反查（窗口内的三段次级别单元即该盘整的构成件）。
    pub windows: Vec<(usize, usize)>,
    /// 操作序列本列：不延伸折叠的走势类型块（盘整+盘整可表示；铺排纪律见模块头）。
    pub blocks: Vec<MoveBlock>,
}

/// 横向旁路读法（ADR 0011 裁定一/六）：拿第 k 层元素用不延伸规则重折。
///
/// `units`：第 k 层元素（级别循环内现场值——L0 = parser 线段规约单元，L(k≥1) = 上级走势塔的
/// `project_to_units` 投影）。`build`：塔自己的级别 seed 判据（**不另立判据**，#804 总缝规则）。
/// `level`：挂载级别盖章（纯标记，不参与判定）。
///
/// 扫描（口径 S，#826 探针直译转正）：窗口恒 3 段，`build` 成立即取中枢并 `i += 3`（三段被该
/// 盘整消费），不成立 `i += 1`（该单元不进任何中枢——趋势连接段的构成件）。
pub fn operation_decompose(
    units: &[UnitRange],
    build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center>,
    level: u32,
) -> OperationSequence {
    let mut centers: Vec<Center> = Vec::new();
    let mut windows: Vec<(usize, usize)> = Vec::new();
    let mut i = 0usize;
    while i + 2 < units.len() {
        if let Some(c) = build(&units[i], &units[i + 1], &units[i + 2]) {
            centers.push(c);
            windows.push((i, i + 2));
            i += 3;
        } else {
            i += 1;
        }
    }
    let blocks = fold_operation_blocks(&centers);
    OperationSequence {
        level,
        centers,
        windows,
        blocks,
    }
}

/// 不延伸折叠（裁定六：最终产物不延伸）：口径 S 中枢链 → 操作序列块。
///
/// 关系判定 = `classify_relation`（与主干**同一判定**，#804——差异只在合并规则，不在判据）：
/// - Up/Down 关系：maximal 同向 run 折成一个趋势块（趋势定义，非延伸）；
/// - LevelExpansion 关系：**不合并**——未被趋势块吸收的中枢各自成单中枢盘整块
///   （相邻盘整块 = 盘整+盘整，`038:22`）。
fn fold_operation_blocks(centers: &[Center]) -> Vec<MoveBlock> {
    let m = centers.len();
    if m == 0 {
        return Vec::new();
    }
    if m == 1 {
        // PDF §6 盘整块情形1（与 decompose 同例）：单中枢无同向后继 = 单中枢盘整块。
        return vec![MoveBlock {
            start_center: 0,
            end_center: 0,
            kind: MoveKind::Consolidation,
            dir: None,
            status: MoveStatus::Active,
        }];
    }
    let rels: Vec<CenterRelation> = (0..m - 1)
        .map(|i| classify_relation(&centers[i], &centers[i + 1]))
        .collect();
    let mut in_trend = vec![false; m];
    let mut blocks: Vec<MoveBlock> = Vec::new();
    // 趋势 run：maximal 连续同向（Up/Down）关系段 → 一个趋势块 [run_start..run_end]。
    let mut i = 0usize;
    while i < rels.len() {
        let dir = match rels[i] {
            CenterRelation::UpContinuation => Direction::Up,
            CenterRelation::DownContinuation => Direction::Down,
            CenterRelation::LevelExpansion => {
                i += 1; // 不延伸：重叠关系不开块、不并入——两端中枢各自落盘整（见下）。
                continue;
            }
        };
        let mut j = i + 1;
        while j < rels.len() && rels[j] == rels[i] {
            j += 1;
        }
        for c in i..=j {
            in_trend[c] = true;
        }
        blocks.push(MoveBlock {
            start_center: i,
            end_center: j,
            kind: MoveKind::Trend,
            dir: Some(dir),
            status: MoveStatus::Completed,
        });
        i = j;
    }
    // 未被趋势吸收的中枢各自成单中枢盘整块（盘整+盘整连接的素材）。
    for (c, &used) in in_trend.iter().enumerate() {
        if !used {
            blocks.push(MoveBlock {
                start_center: c,
                end_center: c,
                kind: MoveKind::Consolidation,
                dir: None,
                status: MoveStatus::Completed,
            });
        }
    }
    blocks.sort_by_key(|b| (b.start_center, b.end_center));
    if let Some(last) = blocks.last_mut() {
        last.status = MoveStatus::Active; // 链尾块仍随新元素可变（沿用 decompose 状态纪律）。
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::super::super::types::Tick;
    use super::super::center::{center_from_segments, center_from_window};
    use super::*;

    /// 测试单元：方向 + [lo,hi] + 顺序坐标（si=i, ei=i+1 由调用方注入）。
    fn u(dir: Direction, si: usize, lo: Tick, hi: Tick) -> UnitRange {
        UnitRange {
            start_index: si,
            end_index: si + 1,
            direction: dir,
            lo,
            hi,
        }
    }

    /// 在 [2,9] 区间内来回震荡的 6 段（方向交替、任意三连段核心非空）——038:22「延伸成 6 段」
    /// 的最小场景：主干塔会把它们吸收成 1 个延伸中枢，口径 S 必须产 2 个中枢。
    fn oscillating_six() -> Vec<UnitRange> {
        vec![
            u(Direction::Up, 0, 0, 10),
            u(Direction::Down, 1, 2, 10),
            u(Direction::Up, 2, 2, 9),
            u(Direction::Down, 3, 3, 9),
            u(Direction::Up, 4, 3, 8),
            u(Direction::Down, 5, 4, 8),
        ]
    }

    fn kinds(blocks: &[MoveBlock]) -> Vec<(MoveKind, Option<Direction>, usize, usize)> {
        blocks
            .iter()
            .map(|b| (b.kind, b.dir, b.start_center, b.end_center))
            .collect()
    }

    #[test]
    fn empty_and_single() {
        let e = operation_decompose(&[], center_from_segments, 0);
        assert!(e.centers.is_empty() && e.windows.is_empty() && e.blocks.is_empty());
        let one = oscillating_six();
        let s = operation_decompose(&one[..3], center_from_segments, 0);
        assert_eq!(s.centers.len(), 1);
        assert_eq!(s.windows, vec![(0, 2)]);
        assert_eq!(
            kinds(&s.blocks),
            vec![(MoveKind::Consolidation, None, 0, 0)],
            "单中枢 = 单中枢盘整块（PDF §6 情形1）"
        );
        assert_eq!(s.blocks[0].status, MoveStatus::Active);
    }

    /// 038:22 直译锁：「延伸成 6 段 = 两个盘整类型的连接」——同一 seed 判据下，主干塔（延伸
    /// 吸收）吃成 1 个中枢，旁路（恒 3 段禁延伸）产 2 个中枢、2 个相邻单中枢盘整块。
    #[test]
    fn six_segment_extension_becomes_parallel_consolidations() {
        let units = oscillating_six();
        // 主干口径（延伸吸收）：6 段吃成 1 个中枢——对照锚（ recursive_tower 同一扫描）。
        let (tower_centers, _, _) = super::super::recursive_tower::detect_centers_windowed_resume(
            &units,
            center_from_segments,
            0,
        );
        assert_eq!(tower_centers.len(), 1, "主干塔：6 段延伸吸收成 1 个中枢");
        // 旁路口径 S：2 个中枢 + 盘整+盘整。
        let s = operation_decompose(&units, center_from_segments, 0);
        assert_eq!(
            s.centers.len(),
            2,
            "口径 S：6 段 = 两个 3 段窗口 = 2 个中枢"
        );
        assert_eq!(s.windows, vec![(0, 2), (3, 5)]);
        assert_eq!(
            kinds(&s.blocks),
            vec![
                (MoveKind::Consolidation, None, 0, 0),
                (MoveKind::Consolidation, None, 1, 1),
            ],
            "两个相邻单中枢盘整块 = 盘整+盘整连接（038:22）"
        );
        // 两中枢外缘重叠（LevelExpansion）但**不合并**——与 decompose::fold_rel 的规则差锚点。
        assert_eq!(
            classify_relation(&s.centers[0], &s.centers[1]),
            CenterRelation::LevelExpansion
        );
        assert_eq!(s.blocks[0].status, MoveStatus::Completed);
        assert_eq!(s.blocks[1].status, MoveStatus::Active);
    }

    /// 趋势块仍合并（定义性判据，非延伸）：两个依次向上分离的中枢 = 一个趋势块；
    /// 其后接重叠中枢（LevelExpansion）⟹ 趋势块不吞它，它独立成盘整块。
    #[test]
    fn trend_runs_merge_but_expansion_never_merges() {
        // 三个三连窗口：W0 在 [0,10]、W1 整体上移分离（[20,30] 带）、W2 与 W1 重叠。
        let units = vec![
            // W0：core [2,9]，外缘 [0,10]
            u(Direction::Up, 0, 0, 10),
            u(Direction::Down, 1, 2, 10),
            u(Direction::Up, 2, 2, 9),
            // W1：core [22,29]，外缘 [20,30]（dd=20 > W0.gg=10 ⟹ UpContinuation）
            u(Direction::Up, 3, 20, 30),
            u(Direction::Down, 4, 22, 30),
            u(Direction::Up, 5, 22, 29),
            // W2：core [24,31]，外缘 [23,32]（与 W1 外缘重叠 ⟹ LevelExpansion）
            u(Direction::Up, 6, 24, 32),
            u(Direction::Down, 7, 24, 31),
            u(Direction::Up, 8, 25, 31),
        ];
        let s = operation_decompose(&units, center_from_segments, 0);
        assert_eq!(s.centers.len(), 3);
        assert_eq!(s.windows, vec![(0, 2), (3, 5), (6, 8)]);
        assert_eq!(
            kinds(&s.blocks),
            vec![
                (MoveKind::Trend, Some(Direction::Up), 0, 1),
                (MoveKind::Consolidation, None, 2, 2),
            ],
            "同向关系合并成趋势块；LevelExpansion 的两端各自落块、互不并入"
        );
    }

    /// 转折共享边界中枢（与 decompose 一致）：[Up, Down] 关系链 → 两个趋势块共享转折中枢 C1。
    #[test]
    fn trend_turn_shares_boundary_center() {
        // C0 在 [0,10] 带、C1 在 [20,30] 带（Up）、C2 回落到 [5,15] 带（Down，与 C1 分离）。
        let s = operation_decompose(
            &[
                u(Direction::Up, 0, 0, 10),
                u(Direction::Down, 1, 2, 10),
                u(Direction::Up, 2, 2, 9),
                u(Direction::Up, 3, 20, 30),
                u(Direction::Down, 4, 22, 30),
                u(Direction::Up, 5, 22, 29),
                u(Direction::Down, 6, 8, 15),
                u(Direction::Up, 7, 5, 14),
                u(Direction::Down, 8, 6, 13),
            ],
            center_from_segments,
            0,
        );
        assert_eq!(
            kinds(&s.blocks),
            vec![
                (MoveKind::Trend, Some(Direction::Up), 0, 1),
                (MoveKind::Trend, Some(Direction::Down), 1, 2),
            ]
        );
    }

    /// 上级 seed 判据路径（center_from_window，无方向维度）同样禁延伸。
    #[test]
    fn window_seed_path_also_bans_extension() {
        // 上级单元无方向要求：6 个区间互相重叠的单元 → 2 窗口 2 中枢 → 盘整+盘整。
        let units: Vec<UnitRange> = (0..6)
            .map(|i| u(Direction::Up, i, (i % 3) as i64, 10 - (i % 2) as i64))
            .collect();
        let s = operation_decompose(&units, center_from_window, 1);
        assert_eq!(s.level, 1);
        assert_eq!(s.centers.len(), 2);
        assert_eq!(
            kinds(&s.blocks),
            vec![
                (MoveKind::Consolidation, None, 0, 0),
                (MoveKind::Consolidation, None, 1, 1),
            ]
        );
    }

    /// 分解唯一性（038:18/28）：同一输入两跑逐位相同；贪心扫描前缀稳定——截断到前 n 个
    /// 单元的产出 = 全量中完全落在 [0,n) 内的窗口族。
    #[test]
    fn decomposition_is_unique_and_prefix_stable() {
        let mut units = oscillating_six();
        // 再接一段上移分离 + 回落，凑出趋势块 + 盘整块的混合序列。
        units.extend([
            u(Direction::Up, 6, 20, 30),
            u(Direction::Down, 7, 22, 30),
            u(Direction::Up, 8, 22, 29),
            u(Direction::Down, 9, 8, 15),
            u(Direction::Up, 10, 5, 14),
            u(Direction::Down, 11, 6, 13),
        ]);
        let a = operation_decompose(&units, center_from_segments, 0);
        let b = operation_decompose(&units, center_from_segments, 0);
        assert_eq!(a, b, "同一主干序列，旁路重折结果唯一（两跑逐位相同）");
        // 前缀稳定：截断到 9 个单元（前 3 窗口恰完整）。
        let pre = operation_decompose(&units[..9], center_from_segments, 0);
        let inside: Vec<(usize, usize)> =
            a.windows.iter().filter(|&&(_, e)| e < 9).cloned().collect();
        assert_eq!(
            pre.windows, inside,
            "截断产出的窗口族 = 全量落在前缀内的窗口族"
        );
        assert_eq!(pre.centers.len(), 3);
    }
}
