//! #648 T1 主题件：operation（自 classifier/mod.rs 内联 `mod tests` 纯移动抽离，零行为变更）。

use super::super::super::types::MoveKind;
use super::super::*;
use super::fixtures::*;

/// #881 S3 验收①：旁路读法 API + 至少两个操作级别的操作序列产出。
#[test]
fn operation_bypass_two_levels_produce_sequences() {
    let cfg = ThetaConfig::default();
    let layer = operation_oscillating_layer();
    let (out, ops) = classify_with_operations(&layer, &cfg, &[0, 1]);
    assert!(
        out.levels.len() >= 2,
        "夹具须至少构到 L1（两个操作级别挂载的前提）"
    );
    assert_eq!(ops.len(), 2, "两个挂载级别各产一份操作序列");
    assert_eq!(ops[0].level, 0);
    assert_eq!(ops[1].level, 1);

    // L0 旁路：恒 3 段窗口 ⟹ 4 窗口 4 中枢，窗口两两相接（(0,2),(3,5),(6,8),(9,11)）。
    let l0_op = &ops[0];
    assert_eq!(l0_op.windows, vec![(0, 2), (3, 5), (6, 8), (9, 11)]);
    assert_eq!(l0_op.centers.len(), 4);
    // 中枢两两重叠（核心亦重叠，#898 四态起为 CoreOverlap 延伸）但**不合并**：4 个相邻单中枢盘整块
    // = 盘整+盘整+盘整+盘整（038:22，主干 fold_rel 永产不出相邻盘整块）。
    assert_eq!(l0_op.blocks.len(), 4);
    for (i, b) in l0_op.blocks.iter().enumerate() {
        assert_eq!(
            (b.kind, b.dir, b.start_center, b.end_center),
            (MoveKind::Consolidation, None, i, i),
            "每中枢独立成单中枢盘整块（不延伸）"
        );
    }
    for w in l0_op.blocks.windows(2) {
        assert_eq!(
            (w[0].kind, w[1].kind),
            (MoveKind::Consolidation, MoveKind::Consolidation),
            "相邻盘整块 = 盘整+盘整连接（同级别分解明令允许，非同级别分解禁止）"
        );
    }
    // 规则差对照锚：主干 L0 moves 是同一份元素经延伸折叠的产物——4 中枢全链重叠
    // 被 maximal 合并成 1 个盘整块（ADR 0011 裁定四：合并属构造，不属读法）。
    assert_eq!(out.levels[0].centers.len(), 4);
    assert_eq!(out.levels[0].moves.len(), 1);
    assert_eq!(
        (
            out.levels[0].moves[0].kind,
            out.levels[0].moves[0].start_center,
            out.levels[0].moves[0].end_center
        ),
        (MoveKind::Consolidation, 0, 3),
        "主干：同一延伸链折成 1 块；旁路：4 块并列——延伸/不延伸的规则差"
    );

    // L1 旁路：上级 seed 判据（center_from_window）同规则产出，序列非空且块索引自洽。
    let l1_op = &ops[1];
    assert!(!l1_op.centers.is_empty(), "L1 操作序列须有中枢产出");
    assert_eq!(l1_op.centers.len(), l1_op.windows.len());
    for b in &l1_op.blocks {
        assert!(b.end_center < l1_op.centers.len());
    }
}

/// #881 S3 验收②：分解唯一性测试锁——同一主干序列，旁路重折结果唯一。
#[test]
fn operation_decomposition_uniqueness_lock() {
    let cfg = ThetaConfig::default();
    let layer = operation_oscillating_layer();
    // (a) 同输入两跑逐位相同（038:18 唯一性的机器形态：无路径依赖）。
    let (_, ops_a) = classify_with_operations(&layer, &cfg, &[0, 1]);
    let (_, ops_b) = classify_with_operations(&layer, &cfg, &[0, 1]);
    assert_eq!(
        ops_a, ops_b,
        "同一主干序列，旁路重折结果唯一（两跑逐位相同）"
    );
    // (b) 挂载集合无关性：单挂 L0 vs 乱序重复挂载 [1,0,0]——L0 序列逐位相同
    // （重折不依赖于还挂了哪些级别：规则按角色分，不按级别分，裁定六）。
    let (_, ops_only0) = classify_with_operations(&layer, &cfg, &[0]);
    let (_, ops_mixed) = classify_with_operations(&layer, &cfg, &[1, 0, 0]);
    assert_eq!(ops_only0.len(), 1);
    assert_eq!(
        ops_mixed.len(),
        2,
        "重复/乱序挂载声明归一化（排序去重），不重复产序列"
    );
    let l0_in_mixed = ops_mixed.iter().find(|o| o.level == 0).unwrap();
    assert_eq!(
        ops_only0[0], *l0_in_mixed,
        "同一主干序列的旁路重折与挂载集合无关（唯一性）"
    );
}

/// #881 S3 验收③：不回流测试锁——旁路产物写入后主干状态逐位不变。
#[test]
fn operation_bypass_no_backflow_lock() {
    let cfg = ThetaConfig::default();
    let layer = operation_oscillating_layer();
    let baseline = classify(&layer, &cfg);
    let (with_ops, ops) = classify_with_operations(&layer, &cfg, &[0, 1]);
    assert!(!ops.is_empty(), "旁路确有产物写入（锁的前提）");
    assert_eq!(
        baseline, with_ops,
        "旁路产物写入后主干 Classification 逐位不变（裁定七：旁路结果不回流主干）"
    );
    // 既有入口逐字节不动（空挂载 = 旧行为）。
    let (plain, _) = classify_with_tower(&layer, &cfg);
    assert_eq!(baseline, plain);
}
