# L2-dist：段2 全历史 depth/小转大分布（level1-4 Type2/3）——结果包

- task: #23（acc-optB-l2dist）
- owner: ws-l2dist
- date: 2026-07-02
- **认识论等级**：**L2**（真实 BTC 全历史逐信号结构下钻，确定性 div_cand，产出否定性计数）
- 方法学：`.chanlun/review-results/cand-nest-stage2-20260702.md` §3.4
- 运行环境：HEAD（6fba4696e8）隔离副本 `/tmp/l2dist2`（共享工作树当时被并发 stroke.rs
  半成品编译红——`last_fractal` 3 处初始化器缺失；隔离保证纯段2基线测量，无并发 WIP 混入）
- 运行：release，813.67s，BTC 全历史 4,613,599 bars（2017-08-17→2026-05-31，max_bars=∞）
- 原始数据：`/tmp/l2dist2/.chanlun/review-results/l2-depth-raw-20260702.md`（collector 落盘，本文件为其六要素化）
- 收集器：临时 `#[test] #[ignore] l2_depth_distribution_dx`（econ_positive.rs 叠加于隔离副本，
  **非交付、不入生产路径**；复用 h2_sample_exclusion_dx 的 bit-exact classify 循环 + 私有
  `descend_type1_anchor_depth` 直调）

## 1. 结论

对 level1-4 每条 per-delta Type2/3 信号（`!is_type1 && (is_type2||is_type3)`，与
`build_nest_certificate` 的 per-delta 判型同源）跑 `descend_type1_anchor_depth`：

### 1.1 各 level 分布

| lvl | 信号总数 | base_none(无候选段) | 小转大(descend None) | 有锚(Some d) | 小转大% |
|---|---|---|---|---|---|
| 1 | 1059 | 0 | 966 | 93 | 91.22% |
| 2 | 324 | 0 | 304 | 20 | 93.83% |
| 3 | 69 | 0 | 64 | 5 | 92.75% |
| 4 | 21 | 0 | 19 | 2 | 90.48% |
| **合计** | **1473** | **0** | **1353** | **120** | **91.85%** |

### 1.2 depth 直方图（descend=Some(d)）

| lvl \ d | d=1 | d=2 | d=3 | d≥4 |
|---|---|---|---|---|
| 1 | 93 | 0 | 0 | 0 |
| 2 | 16 | 4 | 0 | 0 |
| 3 | 4 | 0 | 1 | 0 |
| 4 | 2 | 0 | 0 | 0 |

max_depth 观测 = 3（level3 一例真递归下沉 3 级）；无溢出。

### 1.3 锚点正确性抽样

120/120 全通过（Some(d) 结果的锚段满足 `end_index==source_index` ∧ 方向=−δ 回抽方向；
抽样 cap=200，实际 Some 总数 120 → **全覆盖，非抽样**）。

### 1.4 真封（计数不变量）

逐 level 三桶穷举 `base_none + 小转大 + Σdepth = 信号总数` 全过；合计 1473 = 0 + 1353 + 120。

### 1.5 核心读数

1. **小转大剔除率 91.85%**（1353/1473，四个 level 均匀 90–94%）：绝大多数 Type2/3 信号的
   精确点回抽段在次级别**无一类背驰锚点**——段2 下沉锚定把这些从区间套证书域中剔除
   （它们是"小转大"式转折，非定律一下沉可定位的对象）。
2. **可锚合格域 8.15%**（120 条），depth 高度集中于 d=1（115/120=95.8%）；
   depth≥2 仅 5 例（level2 四例 d=2、level3 一例 d=3）——真递归下沉存在但稀有。
3. **base_none=0**：每条 Type2/3 信号在 tower[lvl] 中都有 `end_index==source_index`
   候选段——塔与 bsp 的坐标一致性在全历史上无一例断裂。
4. **残差群=0**：Γ 定向后 `!is_type1` 域与 Type2/3 主群完全重合（无"带 δ 但该方向
   无 type2/3 bit"的信号通过 Γ 非 Flat 定向）——报告口径=生产门 descent 触发域，无遗漏。

## 2. 定义依据

- **定律一下沉锚定**（第17课 L66）：Type2/3 买卖点由次级别走势的第一类买卖点构造——
  下沉即在 `s.sub_moves` 中找精确点（`end_index==source_index`）段跑完整 `div_cand`
  （段2 实装，econ_positive.rs `descend_type1_anchor_depth`，与 N^δ 上钻同一判据，非平行简化版）。
- **区间套定理有效域=Type1**（606号）：区间套原文对象是趋势背驰段；Type2/3 必须经
  下沉锚到次级别 Type1 才获得区间套证书——锚不到（None）= 小转大（第43课：小级别转大级别，
  无本级背驰段可套）。
- **群定义**：per-delta 判型与 `build_nest_certificate` 同源（`bits.buy1/sell1` 按 δ 侧取），
  Type1 走本级 `div_cand` 不下沉，非本报告对象。

## 3. 边界条件（结论何时翻转）

1. **单标的单粒度**：仅 BTC 1m 全历史。其他标的/粒度上小转大占比可能偏离 91.85%
   （但四 level 内部一致性 90–94% 提示结构性而非窗口伪影）。
2. **ThetaConfig::default() 依赖**：MACD 参数/tick 量化变更会移动 `div_cand` 判据，
   计数随之变化（方向性结论——小转大占绝对多数——需 >10 倍参数敏感度才翻转）。
3. **收集器域=首次出现即计**：`seen` 按 `(lvl, source_index, bsp_disc)` 去重，取信号
   首次成形时的塔快照。若改为"最终塔回看"（收盘后重算），个别信号的 sub_moves 结构
   可能不同（增量 vs 全量塔的尾部差异），边缘计数会漂移；主分布不受影响
   （h2 先例同口径，bit-exact 对齐生产 descent 时点语义）。
4. **depth≥2 稀有性**（5/120）：若后续 level5+ 出现或数据延长，真递归深度分布可能加厚；
   当前"d=1 主导"结论仅覆盖观测到的 level1-4。

## 4. 下游推论

1. **对 acc-optB（区间套证书门）**：段2 机制生效后，level1-4 Type2/3 的区间套证书
   供给收缩到 8.15%——任何依赖"Type2/3 也有证书"的下游门（如 FullNest 全嵌套通行）
   在这些 level 上实际吞吐由 120 条决定，需按此量级预算统计功效。
2. **对小转大处理**：1353 条被剔除的信号不是废弃物——按第43课它们是"小级别转大级别"
   候选，若未来实装小转大专用通道（区别于区间套通道），这是其精确的输入域清单。
3. **对 673号修复验收**：段1 全 0（depth 概念不存在）→ 段2 depth≥1 实例 120 条 +
   d≥2 真递归 5 条，修复的可观测差异在全历史上坐实。
4. **对塔一致性**：base_none=0 是 tower/bsp 坐标契约的全历史 L2 验证（免费副产品）。

## 5. 谱系引用

- 673号（Cand^δ_ℓ 范围误用——段2 是其修复第二段）
- 606号（区间套有效域=Type1）
- 定律一（第17课 L66）、区间套定理（第27课 L43）、小转大（第43课）
- 与 `project_interval_nesting_not_called_in_backtest`（跨级 4.6%、max 深度=1 的早期读数）
  方向一致且更精确：本次按 bsp 类型分域后，可锚域 8.15%、depth=1 主导 95.8%。

## 6. 影响声明

- **零生产代码改动**：收集器只存在于 `/tmp/l2dist2` 隔离副本（HEAD 基线 + 叠加测试），
  主仓库 econ_positive.rs 的同名测试草稿**不作为交付**（工作树当时编译红非我引入，
  该草稿与隔离副本版本相同，留在工作树中未 commit，可由 Lead 决定去留）。
- **无 git 操作**：未 commit/reset/stash/checkout。
- **新增文件**：本报告 + 隔离副本内原始数据（/tmp，会话后可清理）。
- **数据影响**：为 acc-optB-l2dist 提供 L2 级分布证据；不改变任何已结算定义。
