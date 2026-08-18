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
//! - **折叠不合并重叠关系**（LevelExpansion 扩展 / CoreOverlap 延伸，#898 四态）：
//!   [`decompose::fold_rel`] 的 maximal 同标签连段合并属构造
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
//! `mod.rs` 的集成测试锁（`classify` vs `classify` 的 `Classification` 全等）
//! 承担，不靠本模块自觉。
//!
//! ## 现状与遗留
//!
//! - 本模块只接**全量** `classify` 族（`classify` 入口）。增量塔（TowerCache）
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
/// - 重叠关系（LevelExpansion 扩展 / CoreOverlap 延伸，#898 四态）：**不合并**——未被
///   趋势块吸收的中枢各自成单中枢盘整块（相邻盘整块 = 盘整+盘整，`038:22`）。
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
            level_lift: 0, // 口径 S 同级别分解无升级概念（038:22），lift 恒 0。
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
            CenterRelation::LevelExpansion | CenterRelation::CoreOverlap => {
                // 不延伸：重叠关系（扩展=核心分离∧外缘重叠／延伸=核心重叠，#898 四态）不开块、
                // 不并入——两端中枢各自落盘整（见下）。`038:22`：同级别分解本就不需要延伸/扩展
                // 的概念，两态在此同一处置，级别升档语义归 decompose 的 level_lift。
                i += 1;
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
            level_lift: 0,
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
                level_lift: 0,
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
        // 两中枢外缘重叠且核心亦重叠（#898 四态起为 CoreOverlap 延伸）但仍**不合并**——
        // 与 decompose::fold_rel 的规则差锚点（038:22：同级别分解不需要延伸/扩展概念）。
        assert_eq!(
            classify_relation(&s.centers[0], &s.centers[1]),
            CenterRelation::CoreOverlap
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
            // W2：core [24,31]，外缘 [23,32]（与 W1 核心亦重叠 ⟹ #898 起为 CoreOverlap 延伸）
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
            "同向关系合并成趋势块；重叠关系（延伸/扩展，#898 四态）的两端各自落块、互不并入"
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
    /// ★#902 对拍锁：增量步进（纯追加）每个前缀 == 全量重算。
    #[test]
    fn operation_resume_matches_full_replay_on_appends() {
        let mut seq: Vec<UnitRange> = Vec::new();
        // 混合形态：震荡段（盘整）+ 外缘分离上推（趋势 run）+ 回叠（LevelExpansion 盘整+盘整）。
        let specs = [
            (Direction::Up, 0, 10),
            (Direction::Down, 2, 10),
            (Direction::Up, 2, 9),
            (Direction::Down, 3, 9),
            (Direction::Up, 3, 8),
            (Direction::Down, 4, 8), // 6 段震荡 = 2 盘整中枢
            (Direction::Up, 12, 20),
            (Direction::Down, 14, 20),
            (Direction::Up, 14, 19), // 中枢 B [14,19]（A.gg=10 < B.dd=12：上延续）
            (Direction::Up, 24, 32),
            (Direction::Down, 26, 32),
            (Direction::Up, 26, 31), // 中枢 C [26,31]（再上延续 ⟹ 趋势 run A-B? B-C）
            (Direction::Up, 15, 21),
            (Direction::Down, 16, 21),
            (Direction::Up, 16, 20), // 中枢 D 与 C 外缘重叠 ⟹ LevelExpansion 断链
        ];
        let mut state = OperationSeqState::default();
        for (i, &(dir, lo, hi)) in specs.iter().enumerate() {
            seq.push(u(dir, i, lo, hi));
            // 纯追加：全序列视为已 sealed（dirty_from = len）。
            let inc =
                operation_decompose_resume(&seq, center_from_segments, 0, seq.len(), &mut state);
            let full = operation_decompose(&seq, center_from_segments, 0);
            assert_eq!(
                format!("{:?}", (inc.centers.len(), &inc.blocks)),
                format!("{:?}", (full.centers.len(), &full.blocks)),
                "前缀 {} 增量 == 全量（centers/blocks 逐字段）",
                i + 1
            );
            assert_eq!(inc.windows, full.windows, "前缀 {} 窗口逐位", i + 1);
        }
    }

    /// ★#1019 HIGH 回归锁：评审穷举反例形态——多尾块 + dirty_from=0 全清 + 趋势反转处
    /// 截尾——增量折叠（含 pop 重折）必须逐字段 == 全量 `operation_decompose`。
    /// 构造：趋势 run（Up 延续链）→ 反转（Down 链）→ 回叠（盘整）→ frontier 回退
    /// 到 k=0 / 反转边界 / 尾窗，逐一对拍。
    #[test]
    fn operation_resume_pop_shapes_match_full() {
        // Up 趋势三中枢（[0,10]/[12,20]/[24,32] 外缘分离链）→ Down 反转两中枢 →
        // 回叠一盘整 → 尾巴（反转边界在 run2 首）。
        let specs: Vec<(Direction, Tick, Tick)> = vec![
            (Direction::Up, 0, 10),
            (Direction::Down, 2, 10),
            (Direction::Up, 2, 9),
            (Direction::Up, 12, 20),
            (Direction::Down, 14, 20),
            (Direction::Up, 14, 19),
            (Direction::Up, 24, 32),
            (Direction::Down, 26, 32),
            (Direction::Up, 26, 31),
            (Direction::Down, 30, 40),
            (Direction::Up, 28, 40),
            (Direction::Down, 28, 39),
            (Direction::Up, 12, 20),
            (Direction::Down, 13, 20),
            (Direction::Up, 13, 19),
        ];
        let seq: Vec<UnitRange> = specs
            .iter()
            .enumerate()
            .map(|(i, &(dir, lo, hi))| u(dir, i, lo, hi))
            .collect();
        let mut state = OperationSeqState::default();
        // 全量喂满 → 多尾块齐备（Up 链 + Down 链 + 盘整尾三块）。
        let _ = operation_decompose_resume(&seq, center_from_segments, 0, seq.len(), &mut state);
        // ① dirty_from=0：全清 + 全量重扫（L0 常态证书形态）——逐字段 == 全量。
        let inc = operation_decompose_resume(&seq, center_from_segments, 0, 0, &mut state);
        let full = operation_decompose(&seq, center_from_segments, 0);
        assert_eq!(
            format!("{inc:?}"),
            format!("{full:?}"),
            "dirty_from=0 全清重扫 == 全量"
        );
        // ② frontier 收缩到趋势反转边界（第 9 单元）：seq 本身截短（parser 回撤模拟），
        // 多尾块（Down 链 + 盘整尾）整段弹窗 + 重折——逐字段 == 截短后的全量。
        let mut seq9 = seq.clone();
        seq9.truncate(9);
        let inc = operation_decompose_resume(&seq9, center_from_segments, 0, 9, &mut state);
        let full = operation_decompose(&seq9, center_from_segments, 0);
        assert_eq!(
            format!("{inc:?}"),
            format!("{full:?}"),
            "反转边界截尾（seq 收缩）== 全量"
        );
        // ③ 再收缩到单中枢尾（6 单元）——趋势块部分覆盖截尾形态。
        let mut seq6 = seq.clone();
        seq6.truncate(6);
        let inc = operation_decompose_resume(&seq6, center_from_segments, 0, 6, &mut state);
        let full = operation_decompose(&seq6, center_from_segments, 0);
        assert_eq!(
            format!("{inc:?}"),
            format!("{full:?}"),
            "单中枢尾截断（seq 收缩）== 全量"
        );
    }

    /// ★#902 对拍锁②：frontier 改写（pop 尾窗 + 回卷重扫）后仍 == 全量重算。
    #[test]
    fn operation_resume_matches_full_after_frontier_rewrite() {
        let base = [
            (Direction::Up, 0, 10),
            (Direction::Down, 2, 10),
            (Direction::Up, 2, 9),
            (Direction::Down, 3, 9),
            (Direction::Up, 3, 8),
            (Direction::Down, 4, 8),
            (Direction::Up, 12, 20),
            (Direction::Down, 14, 20),
            (Direction::Up, 14, 19),
        ];
        let mut seq: Vec<UnitRange> = base
            .iter()
            .enumerate()
            .map(|(i, &s)| u(s.0, i, s.1, s.2))
            .collect();
        let mut state = OperationSeqState::default();
        let _ = operation_decompose_resume(&seq, center_from_segments, 0, seq.len(), &mut state);
        // frontier 改写：末 3 单元被重划（parser frontier 语义模拟）——dirty_from 回退。
        seq.truncate(6);
        let _ = operation_decompose_resume(&seq, center_from_segments, 0, 6, &mut state);
        // 新尾段（与旧尾段不同的形态）追加。
        for (i, &(dir, lo, hi)) in [
            (Direction::Up, 30, 40),
            (Direction::Down, 32, 40),
            (Direction::Up, 32, 39),
        ]
        .iter()
        .enumerate()
        {
            seq.push(u(dir, 6 + i, lo, hi));
            let inc =
                operation_decompose_resume(&seq, center_from_segments, 0, seq.len(), &mut state);
            let full = operation_decompose(&seq, center_from_segments, 0);
            assert_eq!(
                format!("{:?}", (&inc.centers, &inc.windows, &inc.blocks)),
                format!("{:?}", (&full.centers, &full.windows, &full.blocks)),
                "frontier 改写后追加 {}：增量 == 全量逐字段",
                i + 1
            );
        }
    }

    #[test]
    fn truncate_pops_all_out_of_bounds_tail_blocks() {
        // 4 窗口 4 中枢：c0→c1 上延续（趋势），c1→c2、c2→c3 外缘重叠（盘整+盘整）。
        let units = vec![
            // W0：外缘 [0,10]
            u(Direction::Up, 0, 0, 10),
            u(Direction::Down, 1, 2, 10),
            u(Direction::Up, 2, 2, 9),
            // W1：外缘 [20,30]（dd=20 > c0.gg=10 ⟹ UpContinuation）
            u(Direction::Up, 3, 20, 30),
            u(Direction::Down, 4, 22, 30),
            u(Direction::Up, 5, 22, 29),
            // W2：外缘 [24,32]（与 c1 重叠 ⟹ LevelExpansion）
            u(Direction::Up, 6, 24, 32),
            u(Direction::Down, 7, 24, 31),
            u(Direction::Up, 8, 25, 31),
            // W3：外缘 [26,33]（与 c2 重叠 ⟹ LevelExpansion）
            u(Direction::Up, 9, 26, 33),
            u(Direction::Down, 10, 26, 32),
            u(Direction::Up, 11, 27, 32),
        ];
        let mut state = OperationSeqState::default();
        let seeded =
            operation_decompose_resume(&units, center_from_segments, 0, units.len(), &mut state);
        assert_eq!(
            kinds(&seeded.blocks),
            vec![
                (MoveKind::Trend, Some(Direction::Up), 0, 1),
                (MoveKind::Consolidation, None, 2, 2),
                (MoveKind::Consolidation, None, 3, 3),
            ],
            "前提：4 中枢 = 趋势 + 盘整+盘整（两个尾块悬在截断点后）"
        );
        // frontier 回卷到 6 单元：窗口 (6,8)/(9,11) 弹出，k=2——截断点后有**两个**尾块。
        let trunc = &units[..6];
        let inc = operation_decompose_resume(trunc, center_from_segments, 0, 6, &mut state);
        let full = operation_decompose(trunc, center_from_segments, 0);
        assert_eq!(
            format!("{:?}", (&inc.centers, &inc.windows, &inc.blocks)),
            format!("{:?}", (&full.centers, &full.windows, &full.blocks)),
            "多尾块截断：增量 == 全量逐字段（无悬垂残留）"
        );
        // 截后把原尾段追加回来（续扫路径）仍 == 全量。
        let inc2 =
            operation_decompose_resume(&units, center_from_segments, 0, units.len(), &mut state);
        let full2 = operation_decompose(&units, center_from_segments, 0);
        assert_eq!(
            format!("{:?}", (&inc2.centers, &inc2.windows, &inc2.blocks)),
            format!("{:?}", (&full2.centers, &full2.windows, &full2.blocks)),
            "截断后原样续扫：增量 == 全量逐字段"
        );
    }

    #[test]
    fn dirty_from_zero_full_clear_does_not_accumulate_blocks() {
        // 同 trend_turn_shares_boundary_center 形态：c0→c1 Up、c1→c2 Down（两趋势块）。
        let units = vec![
            u(Direction::Up, 0, 0, 10),
            u(Direction::Down, 1, 2, 10),
            u(Direction::Up, 2, 2, 9),
            u(Direction::Up, 3, 20, 30),
            u(Direction::Down, 4, 22, 30),
            u(Direction::Up, 5, 22, 29),
            u(Direction::Down, 6, 8, 15),
            u(Direction::Up, 7, 5, 14),
            u(Direction::Down, 8, 6, 13),
        ];
        let full = operation_decompose(&units, center_from_segments, 0);
        let mut state = OperationSeqState::default();
        let _ =
            operation_decompose_resume(&units, center_from_segments, 0, units.len(), &mut state);
        // 同输入以 dirty_from=0 连调两次（全清 + 重扫两轮）——每轮都必须 == 全量，无重复块。
        for round in 1..=2 {
            let inc = operation_decompose_resume(&units, center_from_segments, 0, 0, &mut state);
            assert_eq!(
                format!("{:?}", (&inc.centers, &inc.windows, &inc.blocks)),
                format!("{:?}", (&full.centers, &full.windows, &full.blocks)),
                "dirty_from=0 第 {round} 轮：增量 == 全量逐字段（块链不累积）"
            );
        }
    }

    #[test]
    fn truncate_at_trend_reversal_leaves_boundary_as_prev_trend_tail() {
        let base = vec![
            // c0 外缘 [0,10]；c1 外缘 [20,30]（Up）；c2 外缘 [5,15]（Down）⟹ [趋势U 0-1, 趋势D 1-2]
            u(Direction::Up, 0, 0, 10),
            u(Direction::Down, 1, 2, 10),
            u(Direction::Up, 2, 2, 9),
            u(Direction::Up, 3, 20, 30),
            u(Direction::Down, 4, 22, 30),
            u(Direction::Up, 5, 22, 29),
            u(Direction::Down, 6, 8, 15),
            u(Direction::Up, 7, 5, 14),
            u(Direction::Down, 8, 6, 13),
        ];
        let mut state = OperationSeqState::default();
        let seeded =
            operation_decompose_resume(&base, center_from_segments, 0, base.len(), &mut state);
        assert_eq!(
            kinds(&seeded.blocks),
            vec![
                (MoveKind::Trend, Some(Direction::Up), 0, 1),
                (MoveKind::Trend, Some(Direction::Down), 1, 2),
            ],
            "前提：趋势反转形态（c1 为共享边界中枢）"
        );
        // 截到 6 单元（k=2）：趋势 D [1,2] 截尾到单中枢 c1，但 c1 已是趋势 U 的尾 ⟹ 整块丢弃。
        let trunc = &base[..6];
        let inc = operation_decompose_resume(trunc, center_from_segments, 0, 6, &mut state);
        let full = operation_decompose(trunc, center_from_segments, 0);
        assert_eq!(
            kinds(&full.blocks),
            vec![(MoveKind::Trend, Some(Direction::Up), 0, 1)],
            "全量对照：截断后 c1 留作趋势尾，无盘整块"
        );
        assert_eq!(
            format!("{:?}", (&inc.centers, &inc.windows, &inc.blocks)),
            format!("{:?}", (&full.centers, &full.windows, &full.blocks)),
            "趋势反转切点截尾：增量 == 全量逐字段（无伪盘整块）"
        );
        // 截后追加与 c1 外缘重叠的新窗口（LevelExpansion）——c2' 独立落盘整，c1 仍是趋势尾。
        let mut extended = trunc.to_vec();
        extended.extend([
            u(Direction::Down, 6, 21, 29),
            u(Direction::Up, 7, 22, 28),
            u(Direction::Down, 8, 23, 28),
        ]);
        let inc2 = operation_decompose_resume(
            &extended,
            center_from_segments,
            0,
            extended.len(),
            &mut state,
        );
        let full2 = operation_decompose(&extended, center_from_segments, 0);
        assert_eq!(
            kinds(&full2.blocks),
            vec![
                (MoveKind::Trend, Some(Direction::Up), 0, 1),
                (MoveKind::Consolidation, None, 2, 2),
            ],
            "全量对照：LevelExpansion 新中枢独立成盘整块"
        );
        assert_eq!(
            format!("{:?}", (&inc2.centers, &inc2.windows, &inc2.blocks)),
            format!("{:?}", (&full2.centers, &full2.windows, &full2.blocks)),
            "截尾后续扫追加：增量 == 全量逐字段"
        );
    }
}

// ══ #902 增量维护（TowerCache per-bar resume 路径旁路，#881 遗留） ═══════════════════

/// 口径 S 扫描的增量状态（与 `TowerCache` resume/frontier 同生命周期，#885 同批失效纪律）。
///
/// 不变量：`centers`/`windows` 1:1 且按窗口起点升序；`blocks` = `fold_operation_blocks(centers)`
/// 的增量维护形态（尾块 Active 其余 Completed）；`scan_i` = 下一待判位置（其前走步均已判定）。
/// 扫描规则与全量同一（seed 判据共享 `build` 入参、窗口恒 3 段、成立 `i += 3`、无延伸吸收）。
#[derive(Debug, Clone, Default)]
pub struct OperationSeqState {
    centers: Vec<Center>,
    windows: Vec<(usize, usize)>,
    blocks: Vec<MoveBlock>,
    /// 下一待判扫描位置（< scan_i 的走步均已判定；扫描无记忆，位置即全部状态）。
    scan_i: usize,
}

/// 增量口径 S 扫描：frontier 维护 + 追加。与全量 [`operation_decompose`] 的关系：
/// **任意 frontier 演化序列下，返回的 `OperationSequence` 逐字段 == 对当前 `units` 全量重算**
/// （对拍锁 `operation_resume_matches_full_replay`）。
///
/// - `dirty_from`：本级 units 的不可变前缀长度（与 TowerCache 同一份证书：L0 = parser
///   前缀、L≥1 = 父级 prefix_count）。读域越界的已产窗口（i+2 ≥ dirty_from）整窗弹出，
///   `scan_i` 回卷到末保留窗口的 i+3——跳过步在不可变单元上重放同判定，bit-exact；
/// - 追加：从 `scan_i` 续扫（恒 3 段、成立 `i += 3`），新中枢经关系逐条增量折叠
///   （趋势 run 延伸/新趋势开块/盘整+盘整，与 `fold_operation_blocks` 同规则）。
pub fn operation_decompose_resume(
    units: &[UnitRange],
    build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center>,
    level: u32,
    dirty_from: usize,
    state: &mut OperationSeqState,
) -> OperationSequence {
    // ① frontier 失效：已判定走步可能读过可变单元（保守触发：scan_i 的上一判定位置
    // 读域 [p, p+2] 与可变区相交）。弹出读域越界窗口并回卷 scan_i——重放段在不可变
    // 单元上决策不变，跨界段按当前单元重判，与全量重算逐位一致。
    if state.scan_i + 1 >= dirty_from {
        while let Some(&(i, _)) = state.windows.last() {
            if i + 2 < dirty_from {
                break;
            }
            state.windows.pop();
            state.centers.pop();
        }
        let sealed_next = state.windows.last().map(|&(i, _)| i + 3).unwrap_or(0);
        if sealed_next < state.scan_i {
            state.scan_i = sealed_next;
        }
        // ★#1019 HIGH 修复（影子评审坐实 truncate_blocks 与全量折叠不等价——多尾块只弹
        // 一块、趋势截尾单中枢伪盘整、dirty_from=0 残留悬垂块）：pop 路径改为对保留中枢
        // **直接重折 fold_operation_blocks**——正确性按构造恢复（它就是全量判据本身）。
        // pop 是 frontier 事件（稀有、尾部界），O(centers)=百级可接受；append 路径仍走
        // append_center 增量折叠（O(1)/bar，保 #902 的 O(n²) 规避）。
        state.blocks = fold_operation_blocks(&state.centers);
    }
    // ② 追加扫描（规则与全量同一）。
    while state.scan_i + 2 < units.len() {
        let i = state.scan_i;
        if let Some(c) = build(&units[i], &units[i + 1], &units[i + 2]) {
            append_center(&mut state.blocks, &state.centers, &c);
            state.centers.push(c);
            state.windows.push((i, i + 2));
            state.scan_i = i + 3;
        } else {
            state.scan_i = i + 1;
        }
    }
    OperationSequence {
        level,
        centers: state.centers.clone(),
        windows: state.windows.clone(),
        blocks: state.blocks.clone(),
    }
}

/// 新中枢到达的增量折叠（与 `fold_operation_blocks` 同规则的增量形态）：
/// `existing` = 追加前的中枢序列（尾中枢 c_j），`new` = 新中枢 c_(j+1)；关系 =
/// `classify_relation(c_j, c_(j+1))`——同向延续：尾趋势块同向则延伸、否则开新趋势块
/// （尾盘整块被吸收回撤）；重叠关系（LevelExpansion 扩展 / CoreOverlap 延伸，#898 四态）：
/// 不合并，新中枢各自单中枢盘整块。
fn append_center(blocks: &mut Vec<MoveBlock>, existing: &[Center], new: &Center) {
    if let Some(last) = blocks.last_mut() {
        last.status = MoveStatus::Completed;
    }
    let j1 = existing.len(); // 新中枢下标
    if j1 == 0 {
        blocks.push(MoveBlock {
            start_center: 0,
            end_center: 0,
            kind: MoveKind::Consolidation,
            dir: None,
            level_lift: 0, // 口径 S 同级别分解无升级概念（038:22），lift 恒 0。
            status: MoveStatus::Active,
        });
        return;
    }
    let r = classify_relation(&existing[j1 - 1], new);
    let dir = r.trend_direction(); // 扩展/延伸（重叠关系，#898 四态）→ None：不并入趋势块。
    match dir {
        Some(d) => {
            let extend = matches!(blocks.last(), Some(b) if b.kind == MoveKind::Trend
                && b.dir == Some(d) && b.end_center + 1 == j1);
            if extend {
                blocks.last_mut().expect("尾块在").end_center = j1;
            } else {
                // 尾盘整块 [j,j] 被新趋势吸收（全量折叠下该中枢不落盘整）——撤块。
                if matches!(blocks.last(), Some(b) if b.kind == MoveKind::Consolidation
                    && b.start_center + 1 == j1 && b.end_center + 1 == j1)
                {
                    blocks.pop();
                }
                blocks.push(MoveBlock {
                    start_center: j1 - 1,
                    end_center: j1,
                    kind: MoveKind::Trend,
                    dir: Some(d),
                    level_lift: 0,
                    status: MoveStatus::Completed,
                });
            }
        }
        None => {
            blocks.push(MoveBlock {
                start_center: j1,
                end_center: j1,
                kind: MoveKind::Consolidation,
                dir: None,
                level_lift: 0,
                status: MoveStatus::Completed,
            });
        }
    }
    if let Some(last) = blocks.last_mut() {
        last.status = MoveStatus::Active;
    }
}
