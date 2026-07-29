# E2E-O 谱系综合（map #529 charting 调研）

日期：2026-07-28　session：#529 charting（Kimi 编排）　状态：未提交（worktree 报告仓）
用途：#529 首批决策票的事实底座。事实性条目均带出处。

> **引用基线注意**：`doc-divergence-endtoend-prototype-20260718.md` 与
> `chanlun/plans/mainline-merged-roadmap-20260717.md` 两文档已被 a080e97b1c 从工作树删除，
> 本报告引其 git 历史版本（锚 commit `640609071d`，下称「原型」「roadmap」）。
> 另：worktree 内 `chanlun/escalate/cert-bsp-binding-ruling-DRAFT-20260717.md` 是 v1 旧版，
> 待裁项准确文本以**主仓 v2** 为准。

## 1. E2E-O 算子与五阶段

`E2E-O(as_of, tower_prefix, prior_state, ManagedBspPolicy) → (Delta, next_state) | InvalidPolicy/InconsistentState/LateAuthorization`，fail-closed；Delta 只含追加量（EventRevision/LineageRevision/ManagedBspCreation/BspLink）非全量快照；同一 `state_as_of` 重跑强制零 Delta（原型:59-61,83；roadmap:60）。

五阶段同读一个 `as_of` 前缀（roadmap:60；原型:63-78）：

1. `ObserveCandidate_at` — 只做 D1-D3 结构候选 + 稳定身份；
2. `ReviseEvent_at` — 按 D4-D5 走状态机（Provisional/Confirmed/Invalidated/Unresolved，终态不复活）；
3. `BuildLineage_at` — 产出 E2E-L 一等谱系（Open/Closed/Invalidated；路径扩展走 `extends_lineage_key` 新 key，原型:69-70,85）；
4. `CloseFloor_at` — E2E-F 分支本地地板（Formal/Quasi/Unresolved；L1=FormalFloor、L0=QuasiFloor，roadmap:63；原型:184）；
5. `Consume_at` — 唯一消费口。

八道缝合线 E2E-S1–S8 = 实证门（谓词/事件/边/谱系/钟/地板/身份/消费，原型:318-325）；**S8 未全过只能称「塔内近似」**（原型:327）。E2E-L 纪律：skip edge 可记、不得伪造中间级别证书（roadmap:64 末段；原型:188）。

## 2. Consume_at 冻结语义与实装状态

冻结语义：「授权 BSP 在**同一生产事务**形成并写入谱系反向边（双向链接），不是给既有 BSP 事后贴标」（roadmap:62；原型:87）；只接收 `Closed` 谱系，以旧 BSP/链接映射保证投影幂等（原型:85）。
签名：`Consume_at(as_of, prior_bsp_by_key, prior_links_by_key, closed_lineage, ManagedBspPolicy) → (ManagedBspCreation[], BspLink[]) | NoConsumption|InvalidPolicy|InconsistentState|LateAuthorization`（原型:75-78）。
**rust 实装状态：worktree `rust/src` 全量 grep `Consume_at|consume_at|ConsumeAt` 零命中——名字尚不存在于生产码。**

## 3. N0–N7 缺口序列（audit §5，nest-tower-native-gap-audit-20260717.md:164-202）

- **N0**：Cand 语义二分未统一（结构宽候选 vs 力度确认）——裁定前置；
- **N1**：背驰段区间无塔内原生载体（只在 C2 seam/sidecar）；
- **N2**：跨级区间包含谓词缺（`is_sub` 只在 nest.rs:65-67）；
- **N3**：跨级链/证书对象缺（塔全是单级证书）；
- **N4**：事件↔BSP 稳定身份边缺（靠 source_index 等值临时拼）；
- **N5**：首证钟缺（judge_at 登记在 p92 bin book，不在塔对象）；
- **N6**：`Sel_Θ` 选择器缺/三序不一；
- **N7**：生产消费点缺（最深缺口：BSP 先行、证书事后标注）。

依赖序（roadmap:79 段）：`N0→N1→N2→N3`；N4/N5 依赖 N1；N6 依赖 N3；N7 依赖 N3–N6。
audit 明言：N0–N7 全是缺口陈述与裁定请求、非设计承诺（audit:243-244）。

## 4. gap-audit 结论

「区间套必要条件塔内原生」核心缺口 = N2 包含谓词、N3 链式证书、N7 消费点（G4/G8 标「缺（核心缺口）」「缺（正主）」，audit:99,140,197-202）。分期建议 = §6 对拍 L1→L6 序（audit:215-234）。

## 5. LEE 边界（multi-level-native-execution-design-20260719.md）

LEE（Consume_ℓ）= `Consume_at` 输出按 `BspKey.formation_level` 的执行层投影：不改 `Consume_at` 生产语义，只把 `ManagedBspCreation` 路由进 Ledger_ℓ（:83,116-120）。LEE M1-M4 已落地（map #59 登记：#308/#309/#310；本次独立核到 M1/M2 锚 = `theta_overlay.rs:90-115` 级别账本 + `level_order.rs` 归因，M3/M4 锚未复核）。LEE 不触碰八道 E2E-S*（:83 末）、不新增口径、跨级只走 Closed 谱系显式投影（:120,165）。

## 6. 条款 9 与待裁项（主仓 v2 DRAFT + ADR-0005）

条款 9 修订版批复文本（docs/adr/0005-nest-import-ban-clause9-ruling.md:7-9）：「区间套必要条件应在递归塔内部原生实现（次级别构件、塔内时钟）；nest 管线保持**独立对照实现**身份，其产物禁止回灌判据 crate」；「两套实现**互相独立**」（#450 实测三套不等值、748 锚点交集为空）。理由 B（收窄）：core 判据不得用 nest 证书或 `turn_class` 回写/过滤/改判同一历史锚点的 `Classification`/`BspBits`；CI import 禁令 = 保守架构防火墙。

待裁项 1（主仓 DRAFT:67-68）：「条款 9 立项与否：区间套必要条件的塔内原生实装（把『外部对照』变为『构造内检查』，规模不小，需要单独任务链）」。
待裁项 3（:76-77）：「级别标定立项与否：exec/top ↔ classifier lvl 对应关系的标定任务（条款 5 前置，也是 #102 归因的依赖）」。
（两项处置见 #529 子票——2026-07-28 charting 已裁。）

## 7. 前瞻件状态（formal/Origin/）

- `SegmentAutoConstruct.lean`（:1-23）：`segmentsOfComplete` 全自动递归切分接完整判据，全 L0、零 sorry，证终止性+唯一性，不声称对应真实行情；入 lakefile roots（formal/lakefile.toml:104）。rust 未实装。
- `BspEventBridge.lean`（:1-58）：薄 `Bsp`↦厚 `ChanlunEvent` 判据真桥（消双投影同源），全 L0 零 sorry（:356）；只覆盖 type1+type3 买，type2 标 still-OPEN。
- 两者本次未跑 `lake env lean` 复验。

## 8. 因果形态（原型 §4.2，:154-161）

**非法近似**四类：①用终态 `divergence_interval`（现行名 `interval_b`）几何配 prefix 首见钟；②以未来完成的比较对象回填 `first_provable_at`；③丢失 `Invalidated` 路径；④把回试段吞入父背驰段。现行「终态几何 + prefix 首证钟」缝合（nest 管线形态）**可作对照实验，不可称全程因果原生**（缝合事实见 audit §3）。合法降延迟 = 同因果前缀内部分面积外推（024:28）、即时平均力度（015:42）、父段行进同步递降（061:26/061:34）、回中枢留作后果审计（024:46）。

## 9. charting 处置登记（2026-07-28）

首批决策点候选 8 条，本 session 处置：立项包（待裁项 1+3）→ grilling 票即决；因果形态封口 → grilling 票即决；级别标定实证 → research 票（AFK）；N0 语义裁定（Cand 二分 + Sub 边判据）→ grilling 票（frontier）；五阶段落地顺序 → 按 roadmap:79 依赖序逐 N 出票（charting 自定，未烤）；Consume_at 签名冻结/LEE 对接门/E2E-S8 验收基线 → 留雾（N3 后具体化）。
