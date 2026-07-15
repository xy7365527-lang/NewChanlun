# 静默双核结裁：econ 投影接缝改读继承核 + 升级切片显式打标（C1 强制，非翻案）

- 日期：2026-07-15　裁决形态：用户授权自主推进（口头指令"你自主推进 不要问我"），Claude 依既有裁定链拟裁并落地；**本裁不翻任何已结算裁定**，若用户事后否决，回滚点 = 本文 §实施 的 version tuple。　记录：Claude（task #90）
- 结裁：**准绳 = 塔层继承核**。econ 投影接缝（`project_extended_windows`）对重切子窗改读 compose 携带核的 [ZD,ZG]，并以 `SeedCoreProvenance` 显式打标（禁静默）；**215 InvalidSeed 维持 D1 选项 0 fail-closed 不收复**；#142 设计内的 dd/gg 外缘差异不在本裁范围。
- 上游材料：
  1. **#89 审计**：`chanlun/review-results/silent-dual-core-audit-20260715.md`（3,169 静默双核；零反例 ×2；下游 166/209 run 已分叉；裁决点 1–3 即本文待裁）
  2. **#148 裁定链**：`.chanlun/review-results/codex-decide-20260704-001933-2831.md`（A1/B-II/C1 维持；C1 理由 :139；边界条件 :143）
  3. **谱系 689**：`.chanlun/genealogy/pending/689-upgrade-9seg-provenance-course33-multiplicity-not-course20-central-theorem2-q2-domain-narrowed.md`（9段升级出处 = 第33课；Q2 有效域分裂）
  4. **#88 预审**：`chanlun/review-results/b-route-recut-precheck-20260715.md`（36/215 = C1 必然副产品；"修"= 重开裁定）
  5. **D1/D2 结裁**：`chanlun/escalate/d1-seed-anchor-d2-episode-fallback-ruling-20260715.md`（选项 0 + 三条款；滑锚否决；收复须独立立项）
  6. **工程约束**：`chanlun/escalate/d3-direction-ruling-20260714.md` 结裁 5（seam 语义变更必须落 version tuple）

---

## 待裁问题（#89 §遗留 原文）

#148 C1（子窗继承父 seed 核）与 econ 层 offset-0 自核在 3,169 窗上给出两套答案，且块分解已实际分歧（166/209 run）。三候选：C1 维持（econ 层改读继承核）｜C2 子窗自核（塔改用子窗三段交）｜C3 双核显式化（子窗打标，econ/塔各自诚实，禁静默）。

## 依据

### 1. 教义：核心冻结在父窗，重切子窗无独立核

- 第20课中枢定理一（`.chanlun/definitions/zhongshu.md:64-65,162`）：核心 [ZD,ZG] 由**前三段**确定，延伸时**保持不变**。重切前的父窗是一个延伸中枢，其唯一合法核 = 父 seed 三段交。
- 第33课 9段升级（谱系 689）：重切子窗是"同一已成立延伸中枢在第33课约定下的**重新解释**"（`recursive_tower.rs:749-786` 注释、codex C1 :90"同一已成立中枢的重新解释"）——重新解释**不产生新核**，核仍是被解释对象（父窗）的冻结核。
- 升级涌现不依赖子窗核互异：上一级 detect 用 `center_from_window` 纯几何判据吃子窗**外缘**（dd/gg 逐切片各异），零跨级注入（B-III 已拒）。故继承核不会退化升级路径——中心定理二的"两中枢重叠"路径与段数升级路径本就是两条独立机制（谱系 689 topo (1) sever）。

### 2. 裁定链：C1 已结算且刚被复证，econ 自核正是 codex 边界条件警告的形态

- codex-decide-20260704 :139：对子窗复验普通 seed"会把 9 段升级打回不确定触发。这等于否定第 33 课的约定语义"——**C2 否决的既有依据**，本裁直接引用，不重开。
- codex :143 边界条件："若 `Center` 数据模型硬性要求每个中枢都必须由本窗口三段交直接证明，则需要先引入明确的 `upgrade-slice` 语义标记，**不能伪装成普通 seed 中枢**。"——econ 层现状（对升级切片跑 offset-0 自核、成功即静默采用）**正是"伪装成普通 seed 中枢"**：3,169 窗拿着与裁定核不同的自核进了生产块分解。本裁的打标要件即执行该边界条件。
- #88：推翻 C1 = 重开裁定，非修 bug。本裁**执行** C1 而非重开。

### 3. 实证：自核已在生产制造无裁定依据的结构

- #89：166/209 run 块分解分叉、方向不同块 676、292 双核窗落 econ Trend 块内。econ 用自核把"同一延伸中枢的重新解释"看成了互异核的序列，凭空产生趋势/方向结构；塔核块才与已裁语义同源。
- 零反例 ×2（`mismatch_normal=0`、`invalid_normal=0`）：普通窗两核恒等，故 econ 改读携带核对普通窗**bit 不变**，变更面精确 = 重切子窗。

## 结裁

**C1 强制 + 显式打标**（= 候选 C1 吸收 C3 的禁静默要件），四条款：

1. **准绳归属**：塔层 compose 携带核为缠论语义准绳；核心冻结发生在**父窗**（定理一 + 第33课重新解释语义）。econ 投影接缝 `project_extended_windows` 的 seed [ZD,ZG] 改读携带核；seed 的 dd/gg、start/end、identity（source_id/首三段坐标）**不动**（#142 设计内外缘差异、D1 焊死锚身份集均不触碰）。
2. **禁静默**：`ExactThreeSeed` 新增 `core_provenance` 显式字段——`SelfConsistent`（自核 ≡ 携带核，普通窗 + 首子窗）｜`InheritedRecut { self_core }`（自核成立但 ≠ 携带核，3,169 窗，保留自核值供审计）。不允许任何路径在两核不等时静默择一。
3. **215 InvalidSeed 不收复**：维持 D1 选项 0 fail-closed（`ProjectionError::InvalidSeed` 原样）。虽然携带核可让它们"产出"，但收复 = D1 条款 2 的"收复通道"，须独立立项走版本化迁移，本裁不夹带。
4. **C2 终局否决**（依据 2 第一条）：如需复活，须先推翻 codex-decide-20260704 C1 本体，走新裁定。

## 实施（d3 结裁 5：seam 语义变更落 version tuple）

- `ProviderVersion::EXTENDED_TO_EXACT_THREE_V1` → **`EXTENDED_TO_EXACT_THREE_V2`**（`"extended-to-exact-three-v2-inherited-core"`）；全部校验位与 version tuple 断言随迁。
- 携带核读取：`window.rmove` = `RMove::Compose { centers, .. }` 的 `centers[0]`（#89 已证与重算 detect 逐窗 bit-equal，`carried_center_match=OK`）；非 Compose 或 centers 空 ⟹ fail-closed 新错误（不得回退自核）。
- 验收：p89 探针重跑，预期 **双核=0、mismatch_normal=0、InvalidSeed=215 不变、恒等式 4589 = 1205 + 215 + 3169 中 3169 全部转为显式 InheritedRecut**。
- 基准迁移挂账：econ 块分解在 ≤166/209 run 上随核对齐而变 ⟹ 下游收益/块基准（`c2-yield-remeasure-20260715.md` 等）须按新 tuple 重测，**独立任务**，不阻塞本裁落地。

## 下游影响

| 项 | 状态 |
|---|---|
| #89 裁决点 1（准绳归属） | 本文关闭：塔层继承核 |
| #89 裁决点 2（C1/C2/C3） | 本文关闭：C1 强制 + 打标；C2 终局否决（条款 4） |
| #89 裁决点 3（影响面） | 变更面 = 3,169 窗 [ZD,ZG] + provenance 字段；215 窗不动（条款 3） |
| codex :143 边界条件（upgrade-slice 标记） | 本文条款 2 执行 |
| D1 三条款 | 全部维持；本裁不引入滑锚、不收复、不动身份集 |
| #89 归因边界 (b)（dd/gg 延伸外缘） | 不在本裁范围，#142 设计内维持 |
| 块/收益基准 | 挂账重测（新 version tuple），独立任务 |
