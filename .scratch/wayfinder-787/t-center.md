Part of #787

## Question

**中枢的教义正本**——第二批概念票第 1 张。清点见 [总缝规则过筛 #809](https://github.com/xy7365527-lang/NewChanlun/issues/809)（报告 `.chanlun/review-results/issue809-doctrine-triage-20260730.md`，commit `cc1ddd7f4f`）。

排第 1 是因为中枢是**全局输入**（走势类型、买卖点、区间套全都吃它），且全仓唯一那条合规例外候选的退场条件要本票补。

### 要裁的 7 组（Z-1…Z-7）

| 组 | 问题 |
|---|---|
| **Z-1** | 核心区间取哪几段——全三段 `max(3lo)/min(3hi)`（6 处实现）↔ **第 1、3 段**（`a_center_v0.py:145`、`CenterConstruct.lean:490`）。Lean `native_decide` 已钉反例 `(11,18)` vs `(12,15)`。 |
| **Z-2** | 单点核心 `ZD==ZG` 成不成立——Rust 严格 `<` ↔ Lean 弱 `≤`。**按 [#793](https://github.com/xy7365527-lang/NewChanlun/issues/793) 这是「裁错了层」**（端点归 Lean 管），不是跟进不及时。**已有既定前提**：编排者已表态「单点核心必须有宽度」（严格 `<`），本票不重议该立场，要裁的是 **Lean 侧怎么跟**。 |
| **Z-3** | 中枢成立要不要方向判据——方向交替 ↔ s1/s3 同向不查 s2 ↔ 完全不查（三种，5+ 处）。**已记账**（[#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 明记归本票）。 |
| **Z-4** | 有无 ≥9 段升级重切——有（`recursive_tower.rs:57` `UPGRADE_TOTAL_SEGMENTS=9`，第 33 课）↔ 无上限（5 处）。**同段数据中枢个数不同 ⟹ 走势类型跟着翻转。** |
| **Z-5** | 中枢结算后从哪继续扫——四种（`max(break−2,seg_end)` / `last+2` / `j` / `seg1+1`）。 |
| **Z-6** | 有无「候选/确认」中间态——`a_center_v0.py:179` 独有可调 `sustain_m` ↔ 其余几何成立即成立、零参数。 |
| **Z-7** | 成员数固定 3 还是可配置 / 中枢建在什么对象上——固定 3（全部笔段实现）↔ `a_ph_zhongshu.py:329` 可配置、输入是 persistence barcode。 |

### 本票必须一并交付的两件

1. **E1 例外的退场条件**——`compose_level`（`recursive_tower.rs:304`/`:859`）以 `if is_l0` 分派 `center_from_segments`（含方向交替）↔ `center_from_window`（无）。其「诚实有效域」举证书（`center.rs:235`）按 [#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 裁定二**理由够格但缺退场条件、载体在注释非定义文件 ⟹ 现状不合规**。本票须把举证书补齐并搬进 `.chanlun/definitions/`，或裁定收敛掉这条缝。
   ⚠ **例外上限 3 条，现用 1 条**；[#809](https://github.com/xy7365527-lang/NewChanlun/issues/809) 预警本票与背驰票各有一条同型候选，两张票之内可能用光。
2. **正本落 `.chanlun/definitions/zhongshu.md`** + 受影响代码清单（点名到行号），按 `AGENTS.md`「缠论教义正本」关票判据。

### 纪律

`/grill-with-docs`，**一次一问**，agent 不得替人答。落完文档才关票。
