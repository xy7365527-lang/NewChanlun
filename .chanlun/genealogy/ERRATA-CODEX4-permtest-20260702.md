# errata — codex 裁决④ perm_test 两条（已解决 trace · 待 /ritual 编号归档）

**产出者**：genealogist
**日期**：2026-07-02
**上游**：codex 裁决④档案（commit 1aa5e33667）；`.chanlun/review-results/codex-permtest-correctness-ruling-20260702.md`
**同批**：与 `.chanlun/genealogy/RITUAL-STAGING-CODEX-CHOICES-20260702.md`（裁决①10条）同批交 /ritual
**先例**：`settled/612-xianduan-termination-semantics-doc-only-correction.md`（errata 型 doc-correction 谱系记录惯例）

> 两条均为**已解决的 errata trace**（性质异于裁决①10条的「待裁决」）——bug 已修/缺口已 moot，实质已结；仅编号归档待 /ritual（统一编号空间，避碰撞）。留痕目的=免未来误归因（谱系免疫功能）。

---

## errata-1 — perm_test 冻结种子 bug（缺陷-修复事件，estimand 未变）

- **状态**：已解决（errata trace）· 待 /ritual 编号
- **类型**：bias-correction（防误分类：实装 bug ≠ 方法学修订）
- **codex 裁决④**：实装 bug 修复，**不改 estimand**，留 errata **不标方法学修订**
- **权威链**：编排者委托 codex 裁决④；git 历史双坐实（bug 提交 + 修复提交）
- **事实链**：bug 真实存在于创世提交 `8ccbe58137`（HashMap 分组驱动 RNG 消耗序，分组顺序不确定 → 置换种子消耗序不可复现）→ 同日 `7682aa4024` 改 BTreeMap 修复 + 加 `multi_stratum_reproducible` 测试。
- **推导链**：RNG 消耗序 bug 影响的是**复现性**（同输入不同 run 消耗序漂移），不是**被估量本身**（估的是置换分布下的 μ̂ 分位）→ 修复恢复复现性、不改 estimand → 属实装缺陷修复，非方法学修订（若误标方法学修订，会错误连累 8ccbe→7682aa 之间的所有结果口径）。
- **谱系链接**：alpha 簇（645 根 / 663 判据 / 665）；记忆 `project_perm_p_producer_implemented`（8ccbe58137 perm_p 生产者实装）；W-VERIFY 11桶 Inconclusive（晚于修复，不受影响）。
- **topo_effect**：无否定编号节点（确认型：estimand 未变，消解「方法学修订」误分类顾虑）→ N/A
- **影响**：`perm_test.rs`（已修，BTreeMap + multi_stratum_reproducible 测试已加，commit 7682aa4024）；acc-alpha 官方口径不受影响。

## errata-2 — 7682aa4024 溯源缺口（74笔/5桶数字产自修复前/后不可确证 · 已 moot）

- **状态**：已解决（moot · 留痕）· 待 /ritual 编号
- **类型**：source-tracing（溯源完整性缺口）
- **codex 裁决④**：缺口本身留痕；已 moot（不依赖）
- **权威链**：编排者委托 codex 裁决④；git 历史（单提交混装无法拆分）
- **事实链**：`7682aa4024` **同一提交**同时含「修复代码」与「L0三买 VALIDATED（74笔/5桶）」结果 → 无法从 git 历史确证该 74笔/5桶数字产自修复前或修复后的 RNG 消耗序。
- **推导链**：数字与修复混装同提交 → 溯源不可拆分 → 但该数字已被 Geyer 校正的 11桶 Inconclusive 取代，acc-alpha 官方口径不依赖它 → 缺口 moot，然留痕以防未来有人误引 74笔/5桶为 clean 结果。
- **谱系链接**：errata-1（同提交 7682aa4024）；`acc-alpha-estimand-prereg-20260701.md`（官方口径=11桶 Inconclusive）。
- **topo_effect**：无否定边 → N/A
- **影响**：文档层留痕（74笔/5桶 标记为溯源不可确证·已被 11桶 Inconclusive 取代，不得引为 clean）。跨进程复现测试缺口另立 task #41 独立修复（非本 errata 范围）。

---

## /ritual 动作

两条 errata 实质已结（bug 已修 / 缺口已 moot），/ritual 仅需：统一编号归档 → 迁 settled/（或 archive/）。无需编排者价值判断（非选择类）；无 UNDECIDABLE 子问题。
