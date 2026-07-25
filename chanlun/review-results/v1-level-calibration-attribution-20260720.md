# V1 级别标定断裂归因：p107/p108 精读 + 91 证书全绑 lvl=0 机制归因（2026-07-20）

任务：V1 级别标定断裂归因报告（设计文档工位，只读；不改 rust/src，不 git mutation）。
代码行号锚均为主仓 main 分支当前行号；文档锚为 main 分支 blob 行号。

## 0. 材料与 commit 锚

| 材料 | commit | 文件 |
|---|---|---|
| task-107 初版（91 证书全绑 lvl=0，同级不对应） | `f49475510e` | `chanlun/review-results/p107-level-calib-20260717.md` + `rust/src/bin/p107_level_calib.rs` |
| task-107 修正（P107 全级别扫描 lvl0-4 距离矩阵 + L0NN 嵌套基线） | `d32f6d8b2b` | 同上 md（31+/8-） |
| task-107 二次修正（撤回"不是同一标度"，降级为 bar 距离判据下的绑定锚陈述） | `d3ba02dbf3` | 同上 md（6+/1-） |
| task-108（区间套收敛探针） | `e80294b291` | `chanlun/review-results/p108-interval-probe-20260717.md` + `rust/src/bin/p108_interval_probe.rs` |

## 1. p107 核心证据摘要

输入：91 张 nest 证书（A 口径 46 / B 口径 45，来自 #100 对账集），btc_1m_full 全量 4,613,599 bars
（`p107-level-calib-20260717.md:3`）。

- **绑定**：91/91 在 1440 bar 内绑定（`P107_SUMMARY certs=91 bound1440=91`）；b=strong 78 / weak 13
  （:19-20）。exec=1 占 87 张，exec=2 占 4 张（:21、:52-57）。
- **全级别命中**（修正版 `P107_MAT`）：`[0]`×81、`[1]`×4、`[0,1]`×3、`[0,1,2]`×1、`[2]`×1、`[3]`×1；
  `P107_CROSS`：exec=1 → lvl0 n=81 / lvl1 n=8 / lvl2 n=2 / lvl3 n=1；exec=2 → lvl0 n=4（:22-23）。
- **距离矩阵**（`P107_MATRIX`）：exec=1 对 lvl0 strong=71 weak=16 **far=0**；lvl1 far=57、lvl2 far=81、
  lvl3 far=86、lvl4 far=87。**lvl0 是唯一 far=0 的级别**（:29-32）。同级匹配（exec=k vs lvl=k，
  `P107_SAMELVL`）：exec=1 → far=57；exec=2 → far=4——bar 距离判据下无同级对应（:33）。
- **嵌套基线**（`P107_L0NN`）：classifier 自身 lvl≥1 买卖点 ≥97% 也锚在某 lvl0 买卖点附近
  （far 占比 0–4.5%，:39-46）——"落在 lvl0 附近"是层级嵌套的必然，不构成级别对应证据（:48）。
- **二次修正后的有效结论**（:61-66）：bar 距离判据只覆盖"绑定/对账锚"——端点绑定应锚 lvl0；
  lvl≥1 的 far 发散**不构成**对同级对应的否证；同级对应的判定性检验是**区间包含**，未做。
  原表述"不是同一标度（维持）"已撤回。
- **结论 2**（:67）：共同锚点是 lvl=0 端点事件；nest 级别爬升（exec≥2）来自递归证书深度，
  classifier 级别爬升来自窗口合成，**二者判据不同、不可互换**。
- **结论 4/5**（:69-70）：接入裁定建议——绑定一律锚 lvl0 端点；exec/top 随行携带不映射为
  classifier lvl；"级别"一词须区分口径（nest=证书递归深度，classifier=窗口合成级别）。

## 2. p108 核心证据摘要

- 规模：lvl0=18,516 / lvl1=4,472 / lvl2=2,406 / lvl3=1,210 / lvl4=548 事件；(src,side) 多级组仅
  2,456/24,510（10.0%）（`p108-interval-probe-20260717.md:9-10`）。
- **检验 1 同点降级**（:13-24）：严格同 source_index 同侧口径下，「高级别点同时是低级别点」
  miss% = lvl1 70.75 / lvl2 87.78 / lvl3 92.48 / lvl4 91.06——70-92% 缺失。与 p107 L0NN/MAT 一致：
  lvl≥1 点到 lvl0 同侧点是**近邻而非同点**（:22-23）。
- **检验 2 嵌套收缩**（:26-34）：窗口口径（离开窗口）contain/equal = **100%**（86 对，
  partial=disjoint=reversed=0）；中枢口径 81/86 disjoint——非破例，低级别最后中枢是高级别
  离开段内部对象，结构上应当错开。`nocenter` 空洞是 `BspPoint.center` 字段可得性限制（1/2 类点
  可为 None），非结构破例。
- **检验 3 单调收敛**（:36-41）：`P108_MONO` 多级窗口组 114/114 单调收敛（|极值bar−src| 随级别
  下降非增，100%）；`P108_CONV` lvl0 exact0 仅 3.86%（547/14,155）——点≠极值是常态。
- **裁定建议**（:43-50）：①区间套必要条件实装为**窗口式**（lvl k 点确认要求存在 lvl k-1 同侧点
  落在 lvl k 离开窗口内，非同 bar）；②收敛检验用**单调性**不用 exact0；③实装从
  `LevelState.centers` 取区间，消除 nocenter 空洞。

## 3. 91 证书全绑 lvl=0：机制归因

### 3.1 证书级别字段赋值链（代码事实）

1. **事件级号赋值点**：`provide_nest_candidate_events(level: u32, …)`
   （`rust/src/theta_v0/classifier/level_view.rs:646`）把调用方传入的 `level` 写入
   `NestCandidateEvent.level`——Trend 分支 `level_view.rs:713`、Consolidation 分支
   `level_view.rs:783`。调用方以**塔索引**喂入：p92 生产路径 `collect_snapshot_candidates`
   跳过 L0（`rust/src/bin/p92_nest_replay_postruling.rs:564` `if level == 0 || level >= tower.len()`）、
   主循环 `for level in 1..tower.len()`（:650），`by_level[level]` 消费 `tower[level]`
   （p105 §3，`p105-cert-level-spectrum-20260717.md:70`——**nest 体系不监听 L0，exec ≥ 1 恒成立**）。
2. **exec/top 赋值点**：p92 bin 装配环 `for exec in 1..current.len() { for top in exec..current.len() }`
   （`p92_nest_replay_postruling.rs:769-770`）→ `assemble_typed_certificates(events, exec, top, …)`
   （:838）；nest 侧 `exec_level = base.level as usize`（`rust/src/theta_v0/classifier/nest.rs:637`）。
   CERT 行发射 `exec={} top={}`（:877-879）。**exec/top = 塔索引（证书递归深度的两端），
   不是 classifier lvl。**
3. **同基不变量**：`tower_snapshots[i]` 与 `Classification.levels` 同 index 同构
   （`rust/src/theta_v0/classifier/mod.rs:337-341`，`tower_snapshots.len() == levels.len()`）——
   赋值与塔结构一致，无错位。
4. **终端背书账本级别**：`event_bsp_book_level(event_level)`（`nest.rs:487-490`）——
   ℓ≥1 ⟹ Some(ℓ-1)，ℓ=0 ⟹ None。这是 **p117 T1 裁定**（级别移位 ℓ→ℓ-1，
   `chanlun/escalate/bsp-terminal-endorsement-ruling-20260718.md` T1 行；教义锚 037:16/024:18/043:26，
   见 `nest.rs:478-486` 头注），2026-07-18 生效（p92 bin 头注 :3-4）。教义原文已逐字核对：
   024:18「任一背驰都必然制造某级别的买卖点……」（`docs/chanlun/text/blog/024-第24课.md:18`）；
   043:26「由于背驰的级别不可能大于当下走势的级别……」（`docs/chanlun/text/blog/043-第43课.md:26`）；
   037:16「一个趋势，就意味着A、B是同级别的中枢……」（`docs/chanlun/text/blog/037-第37课.md:16`）。

### 3.2 为什么 91 张全绑 lvl=0——三层机制叠加

1. **判据无区分力（主因）**：p107 的绑定判据是 judge_max → 最近 BSP 的 **bar 距离**。
   `P107_L0NN` 基线证明 classifier 自己的 lvl≥1 点也 ≥97% 落在 lvl0 点附近
   （`p107-level-calib-20260717.md:39-48`）——lvl0 密度最高（lvl0=18,516 事件 vs lvl4=548，
   `p108-interval-probe-20260717.md:9`），层级嵌套使任何高级别事件端点在 bar 距离下都
   最近于 lvl0 点。"全绑 lvl0" 是判据的必然输出，**不携带级别信息**。
2. **身份层对应其实存在（反证"断裂"）**：p105 §4 用身份键 `(level, turn_source) ↔
   (lvl, source_index)` 同一坐标系连接，证书标记的去重 BSP 落点为 **L1 21 个 / L2 2 个 /
   L3 2 个 / L0 0 个**（`p105-cert-level-spectrum-20260717.md:18`、:74、:101）——
   在身份粒度上 exec↔lvl 对应**存在**且主体落 L1。L0=0 是结构性事实（nest 不监听 L0），
   不是对应失败。"全绑 lvl0" 只在 bar 距离判据下出现，两种口径回答不同问题
   （p105 :120 已警示不可混用）。
3. **T1 移位使"exec=1 → lvl0"成为设计预测（现行语义）**：91 证书 dump（2026-07-17）早于
   T1 生效（2026-07-18）。在现行语义下，exec=1（87/91）事件的终端背书账本 = `levels[0].bsp`
   （`nest.rs:487-490`）——证书终端正点**本来就是 lvl0 BSP**。`P107_CROSS` 的
   exec=1 → lvl0 n=81/87 恰好是 T1 预测的经验印证，而不是对应失败。p107 初版检验的是
   exec=k vs lvl=k，而 T1 下预测对应是 exec=k vs lvl=k-1——检验设计本身与现行语义错位。

### 3.3 裁定：是赋值 bug 还是语义定义分叉？

**语义定义分叉，不是赋值 bug。** 依据：

- 赋值链（§3.1 第 1-3 条）与塔同构不变量一致，逐级可追溯，无错位、无错赋。
- "断裂"的两个成分各有归属：bar 距离全绑 lvl0 = 判据无区分力（§3.2-1）；exec↔lvl 同级
  不对应 = 两种级别语义（exec/top=塔索引/证书递归深度 vs classifier lvl=窗口合成级别，
  p107 结论 2/5，:67/:70）+ 检验设计未计 T1 移位（§3.2-3）。
- 二次修正（`d3ba02dbf3`）已把初版强结论撤回降级（:61-66）——本报告与该降级一致：
  bar 距离判据下同级对应不可见；**包含判据检验仍是未决项**。
- 090 诚实边界：exec≥2 仅 4 张样本，其中 2 张在数据右缘（:56-57）——exec≥2 的 lvl≥1
  对应**证据不足**，本报告不对其下判定。

## 4. V1 修复方向：改赋值 vs 改语义定义

### 路 A：改语义定义/判据（推荐）

不动任何赋值。内容：(i) "级别"一词分口径显式标注（nest=证书递归深度/塔索引，
classifier=窗口合成级别——p107 结论 5，:70）；(ii) 跨体系级别对齐判据从 bar 距离升级为
**区间包含**（证书 leave/retest 区间 ⊇/⊆ 高级别买卖点所在中枢/背驰段区间——p107 结论 4，
:69）；(iii) 绑定/接入锚维持 lvl0 端点（p107 结论 1/4，:62/:69）。

- 教义依据：037:16（趋势级别=中枢级别——级别由中枢链定义，区间包含是级别的**原生**判据，
  bar 距离不是）；043:26（背驰级别≤走势级别——级别对应天然是向下包含方向，同点/同级
  相等不是教义要求）；024:18（背驰-买卖点定理——级别归属由背书账本定义，T1 已按此移位）。
- 经验基础：p108 检验 2 窗口嵌套 contain/equal=100%（`p108-interval-probe-20260717.md:28-30`）
  ——包含判据在 classifier 侧已 100% 经验成立，判据升级可行。
- 成本：零代码风险；只需在裁定文档与接入层显式口径 + 另立"区间包含对齐"探针任务
  （p107 结论 1 已登记为待做，:64-66）。

### 路 B：改赋值（证书对外级别字段改为结构级别 ℓ-1）

把证书暴露的级号从塔索引改为 T1 结构级别（= 背书账本级别）。

- 教义依据：同 T1（037:16/024:18/043:26，`nest.rs:478-486`）。
- 代价（均为对账链事实，非风格问题）：
  - 破坏 p105 §4 身份键 `(level, turn_source) ↔ (lvl, source_index)` 同坐标系对账
    （`p105-cert-level-spectrum-20260717.md:74`）——ids 是 #98/#99 A/B 跨口径对账主键
    （`p92_nest_replay_postruling.rs:856-857` 侧信道注释）与 `certificate_key` 成分（:1007-1021）。
  - p105 §3 的结构性事实"L0 的 19,776 个 BSP 结构上不可能被证书标记"
    （`p105-cert-level-spectrum-20260717.md:18`、:101）会被翻转——事件层"nest 不听 L0"
    与对外级号"L0 可标记"两种语义在同一字段上相混，正是本报告归因的断裂来源再生产。
  - p117 S1a 已警示：级别映射（ℓ→ℓ-1）× 坐标口径须一次裁定
    （`p117-bsp-repair-design-s1a-20260717.md:184`）；身份层移位超出 T1 授权域（T1 只移
    背书查法，不动事件身份），需编排者新裁定。

### 裁定建议

V1 走**路 A**。路 B 把判据错位误治为字段重定义，会拆掉现有对账链并制造新的口径混配；
"断裂"的病根在判据（bar 距离）与口径（一词两义），不在赋值。若未来确需身份层移位，
按 p117 S1a :184 的二维一次裁定程序另立，不在 V1 范围。

## 5. task-108 同点降级 70-92% 缺失对 V2 力度门设计的影响

1. **禁同点硬判据**：检验 1（`p108-interval-probe-20260717.md:13-24`）⟹ 条款 9（#106）及
   V2 力度门若要求「高级别点/背驰由低级别**同点**背书」，70-92% 的真事件会被误杀——
   这是 p113 实测 30.4% 聋度（`level_view.rs:774` 注释）同类事故的放大版（缺口是 70-92%，
   不是 30%）。必要性条件必须实装为**窗口式**：lvl k 确认要求存在 lvl k-1 同侧点其
   source_index 落在 lvl k 离开窗口内（p108 裁定建议 1，:45-46；窗口嵌套方向 100% 经验成立）。
2. **收敛门用单调性，不用 exact0**：`P108_CONV` lvl0 exact0 仅 3.86%（:39-41）⟹ 力度门不得
   要求「点 bar = 窗口极值 bar」；`P108_MONO` 114/114 单调收敛（:38）给出可实装的替代：
   逐级 |极值距离| 非增（p108 裁定建议 2，:47）。
3. **区间来源取 `LevelState.centers`**：`BspPoint.center` 字段 1/2 类点可为 None（:33-34），
   多级组仅 10%（:10）——力度门若依赖 `BspPoint.center` 会有 nocenter 空洞；应从
   `LevelState.centers`（每级全量中枢序列）取区间（p108 裁定建议 3，:48-50）。
4. **对 divergence_confirmed 现状的定位**：现行力度确认是**级内**判据（R2 力度或关系
   027:32：同色面积 ∨ 黄白线峰 ∨ 同向柱峰，`level_view.rs:773-781`），不跨级——p108 的
   缺失数据不冲击现状；冲击的是 V2 若新增"跨级力度背书"设计。设计 V2 跨级门时，
   1-3 条是硬约束。

## 6. 未决项登记（090：照实否定合格）

- 区间包含判据的同级对应检验**未做**（p107 结论 1 登记，:64-66）——V1 路 A 的 (ii) 即此任务的实装。
- exec≥2 证书的 lvl≥1 对应：样本 4 张、2 张在数据右缘（:52-57），证据不足，不下判定。
- 91 证书 dump 为 T1 生效前（2026-07-17）产物；T1 后重放的全级别扫描未做——
  §3.2-3 的"T1 预测印证"是语义推演 + `P107_CROSS` 旁证，非 T1 后 dump 的直测。
