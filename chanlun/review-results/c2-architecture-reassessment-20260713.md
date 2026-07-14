# C2 架构对抗性再评估（task #66）

- 日期：2026-07-13
- 性质：只读调研 + 必要隔离重放；不修改生产 Rust、Lean、定义或既有裁决
- 结论纪律：本文提出维持、修订或补充裁决的建议，**不执行裁决**；正式变更留给 `escalate`
- 工作区快照：`bed65e9960766e209936e97f476307c399ae22a9`，并叠加工作区内尚未提交的 #60/#61/#63/#64 材料
- 缺口：`chanlun/review-results/seed-failure-geometry-20260713.md`（task #65）在本次读取时不存在。本文将“种子失败中存在划分无关部分，任何架构都救不了”标为**任务给定前提**，不伪造其数量。

## 0. 结论先行

### 0.1 建议

**建议维持 C2 的职责 seam，同时走 `escalate` 补充裁决，不足以修订 C2 本身。** 补充项应至少有两条：

1. 把“塔/消费层职责切分”与“窗口划分策略”拆成两个独立、可版本化的决策：
   - `AssemblySeamVersion`：塔事实与 CompletedMove/Attribution 视图的职责切分；当前维持 C2。
   - `PartitionPolicyVersion`：`greedy_left_to_right_v1`、有界延迟、稳定约束 DP 等划分策略；不得继续把现行贪心扫描默认为 C2 本身。
2. 在现有三道无 repaint 防线外补一条原文门：**同一 `rule_version` 下，真正 `Completed` 的对象不得再改变 chain/membership；普通 supersede 只允许作用于 Pending/临时划分。** 若规则升级要重审，旧版本仍冻结，新版本通过显式 migration/reopen 另起视图。

推荐先走本文提出的 **A4（A0+）**：保留不可变贪心 WindowUnit 塔作为 `v1` 材料源，在消费层把 `PartitionPolicy` 独立版本化并做无回退 shadow replay。A4 不是第五种生产真理，而是对 C2 的最小补充：先验证“换划分”的收益能否在现有 seam 内取得，再决定是否需要把复杂度推入塔。

### 0.2 为什么 #64 尚不足以推翻 C2

1. #64 证明的是**孤儿身份对划分策略敏感**，不是“消费层 seam 错了”。现行贪心扫描的 k1..3 `i+=1` 漏段为 `5,790`；#58 sequence-envelope 组装器已从完整 lower ledger 为其中 `5,477/5,790=94.594128%` 建立 host，只剩 `313` 无 host。事实锚：`q5-orphan-attribution-research-20260713.md:33-46`、`orphan-reduction-paths-20260713.md:72-85`。
2. 换成 #64 的实验 DP 后，旧 313 中确有 136 个获得 host，但目标域同时回退 28 个，故 `313→205` 的净减是 108；在全部 k1..3 lower 上则是 `2,028→2,001`，551 获 host、524 失 host，净减仅 27。事实锚：`orphan-reduction-paths-20260713.md:191-205`。
3. 27 只占旧全域 Unassigned 的 `1.331361%`，占 55,624 个 k1..3 lower 的 `0.048540%`；且生产安全硬门若要求 `regressed_to_unassigned==0`，#64 当前可报告的非零收益仍是 0。事实锚：同文件 `:202-205,218-225`。
4. A2/A3 并未获得 C2 所没有的原始信息：#58/C2 已要求读取完整 lower ledger，而不是只拼塔 `subs`。删除窗口塔或把候选集搬进塔，首先是重新安置复杂度，不自动增加可判事实。事实锚：`assembler-spec-20260712.md:88-106`。

因此，#64 足以否定“贪心划分是唯一自然生产划分”的默认前提，尚不足以否定“稳定事实层 + as-of 解释层”的 C2 seam。

## 1. 证据口径与不能混用的分母

### 1.1 两个对象域

| 对象域 | 数量 | A0 基线 Unassigned | 比例 | 用途 |
|---|---:|---:|---:|---|
| #61 目标域：k1..3 失败漏段 `5,790` + 尾段 `4` | 5,794 | 313 | 5.402140% | 回答“塔漏段后来是否获得 host” |
| 全部 k1..3 lower | 55,624 | 2,028 | 3.645908% | 完全分类与架构净收益的主分母 |

目标域 A0 当前状态为：`Assigned+Completed=1,357`、`Assigned+Pending=4,124`、`Unassigned=313`、`Tombstoned=0`。事实锚：`orphan-reduction-paths-20260713.md:268-279`。

不能把以下数字相加或互换：

- `5,790`：贪心塔失败 `i+=1` 的 raw gap；
- `313`：完整 lower-ledger 组装之后仍无 host 的目标残余；
- `2,028`：全部 k1..3 lower 上的 Unassigned；
- `247`：已经 Assigned 到 139 个 Pending Trend host 的漏段，不是 Unassigned；
- `136`：换划分后旧目标 Unassigned 获 host的毛数；
- `27`：换划分后全 lower 的净减少。

### 1.2 P0-P2 落地后 A0 残余应如何表述

#64 的 P0/P1 正是生产化 #58 完整 lower-ledger chain、延伸与 C 账本；当前原型基线已经包含这些语义。因此，在冻结 `sequence_envelope` 适配的比较口径下：

- **[实测基线]** P0/P1 对应的目标残余是 `U=313`，全域是 `U=2,028`。
- **[推断]** P2 把 139 个结构 Trend 的唯一 direction/A-C/MACD 补齐，主要改变 `Assigned+Pending → Assigned+Completed`。当前有 247 个漏段已在 Pending Trend host 中，P2 没有已证的 Unassigned 减量，故不能把 247 从 313 中相减。
- **[推断]** direction provider 可能改变 grouping 与 membership，故 P2 完成后必须重跑；在重跑前，最诚实的预期仍是目标 `U≈313`、全域 `U≈2,028`，而不是承诺下降。

事实锚：`orphan-reduction-paths-20260713.md:72-85,138-148,324-333`。

### 1.3 #65 依赖变量

定义：

```text
F_ind_target = P0-P2 后目标残余中，经 #65 证明与任何合法划分无关的数量
F_ind_global = P0-P2 后全域残余中，经 #65 证明与任何合法划分无关的数量
```

任务给定前提意味着 `F_ind` 是所有 A0-A3 的不可消除下界；但 #65 文件缺失，本文不赋数值。#64 只能证明 52 个目标 residual 是 exact-three 非种子、换划分后该桶降为 35；它也证明“非种子”不等于“永远无 host”，所以 52 或 35 都不能冒充 `F_ind_target`。事实锚：`orphan-reduction-paths-20260713.md:76-85,191-205,281-287`。

## 2. 关键归责：C2 seam 与贪心扫描是两个决策

| 决策 | 当前实现/裁决 | 它决定什么 | 它不决定什么 |
|---|---|---|---|
| D1：职责 seam | C2：塔产不可变 WindowUnit；消费层产 CompletedMove/Attribution as-of 视图 | 哪一层负责稳定事实、完成解释、版本与 coverage | 三元组冲突时必须选择哪个窗口族 |
| D2：划分策略 | `greedy_left_to_right_v1`：成功 `+3`、失败 `+1` | 哪些三元组成为塔 WindowUnit seed、ordinal/父子谱系 | lower 是否可由消费层延伸、趋势或其他合法 host 吸收 |

源码事实：`detect_centers_windowed` 在成功时 `i+=3`、失败时 `i+=1`，上级 `ElementId` 以该策略的产出 ordinal 编号；增量正确性依赖“已产出窗口永不重访”。锚：`rust/src/theta_v0/classifier/recursive_tower.rs:180-249,252-318`。C2 裁决则只冻结塔/消费层职责和三道防线：`c-ruling-decision-20260713.md:11-24`。

归责结论：

1. **raw `i+=1` gap 的直接原因是 D2 贪心扫描。** 若换合法窗口族，孤儿身份可变，#64 已实测。
2. **raw gap 曾经静默、不能审计的原因是缺消费层完整 lower ledger 与 C 账本。** 这是 D1/消费 seam 的历史缺口；#58/#63 正在修复。
3. **组装后仍剩 313 的原因是当前 grouping/direction/host 与划分策略的合成结果。** 其中多少可归责 D2，须由 #65 和无回退重放分解；不能把 313 全归咎 C2，也不能全归咎种子失败。
4. **Completed coverage 不足的主因还有 A/C hook 与终结证据。** 它与是否有 host 是另一轴。

## 3. 无 repaint 的形式口径

令 `H` 为追加历史，`H ⪯ H'` 表示 `H'` 仅追加未来 lower、候选或事件。令：

```text
V_A(H,t,r) = 架构 A 在 rule_version=r、as_of=t 的完整可见视图
```

第一门的严格形式是：

```text
∀ H ⪯ H', ∀ t ≤ end(H):  V_A(H,t,r) = V_A(H',t,r)
```

等号必须覆盖 partition selection、Move chain、completion、host membership、A/U/T disposition、证据与错误结果；不能只比较窗口数。#58/#63 已给同型契约：`assembler-spec-20260712.md:54-73`、`unassigned-ledger-production-spec-20260713.md:428-479`。

第二门是：

```text
entry_bar ≥ judge_at + 1
```

`judge_at` 必须是该版本选择、完成或 supersede 所需全部证据最晚可知的 source bar，不是图形右端点的别名。

第三门是：旧行不覆盖，变化只通过带 `judge_at` 的追加式 create/supersede/reopen/migration 事件表达；历史查询过滤 `judge_at≤t`，当前查询追随可见版本链。

### 3.1 三门不是原文合法性的充分条件

第69课同时给出更强语义：临时分型可随新材料修改，但“一旦完成的图形，这修改就不可能了”。第70课允许暂时采取有利划分，等走势走出最自然选择后再合理划分。锚：`docs/chanlun/text/blog/069-第69课.md:24-26`、`070-第70课.md:20-40`。

因此仅仅“保留旧 as-of、当前版本 supersede”仍可能让一个已标 `Completed` 的对象在当前视图改变成员。它在数据库意义上不 repaint，却在定义意义上把“完成”降成了 provisional。本文提出第四门：

```text
CompletedFreeze:
同 rule_version 下，一旦 host=Completed，未来事件不得改变其 chain/membership。
规则升级可新开版本，但旧 Completed 仍原样可查。
```

## 4. 架构对比矩阵

表中 `U_target/U_global` 分别指 §1 的两个对象域；所有估算均显式标 `[推断]`。

| 维度 | A0 贪心塔 + 消费组装 | A1 塔内在线非贪心 | A2 无窗口塔 | A3 多候选划分塔 |
|---|---|---|---|---|
| 职责 seam | C2 原案；塔深而窄，消费层解释 | 完成部分选择责任进入塔 | 窗口/走势全部成为消费视图；Lk ledger 来源未定义 | 塔暴露候选搜索空间，consumer 再择一 |
| 前缀稳定 | **强**：现行左折叠窗口前缀不可变；组装器按 as-of 纯归约 | **有条件**：必须延迟 commit、只追加；固定 N 本身不构成证明 | **强但有前提**：所有 Lk ledger 自身须 settled/append-only | **有条件**：候选出生与 selection 都须版本化、按 as-of 过滤 |
| entry 门 | seed 无额外延迟；完成仍等后继/A-C；统一 `+1 bar` | 额外 `N` 个 lower 的确认延迟；bar 延迟数据依赖 | 无人为窗口延迟，但 Completed 递归证据可能更晚 | 候选可早出；法定选择/完成时点可能长期 Pending |
| supersede | 主要在消费层；需补 CompletedFreeze | 若塔改配，塔 ID/父子链也需事件化 | 只在视图/规则迁移层 | 候选集、选择与 host 三层版本，事件量最大 |
| 孤儿实测/预期 | 实测 `313/2,028`；P2 后仍约此数 `[推断]` | 未实测；#64 DP 代理为 `205/2,001`，但有 `28/524` 回退，不能当 A1 承诺 | 未重放；`F_ind` 是硬下界，A0 数是当前参考上界 `[推断]` | 取 greedy 候选即 `313/2,028`；取 #64 选择即 `205/2,001`，数量由 selector 决定 |
| 增量复杂度 | 塔摊销 O(new lower)；纯视图批量约 O(n log n)+O(n)，需 cache adapter 防 per-bar O(n²) | 固定 N 可 O(N) 内存、O(N) 或摊销 O(1) 每步；全局最优无固定 N 保证 | ledger append 便宜；全层视图/递归重建集中到查询，易形成 O(k·n) 或 per-bar O(n²) | 显式 partition 数最坏指数；压成 DAG 可近线性，但 selection DP 回到 consumer |
| Lean/证明资产 | 保留 `canonicalWindows/composeStep`；新增 Window→Completed bridge | 重证延迟 frontier、选择 soundness、ID 稳定和增量等价 | 现有窗口递归降为缓存证明；需重建 Lk ledger/Completed recursion 基础 | 可复用单窗口 soundness；新增候选完备性、选择唯一性、版本单调性 |
| 迁移/可逆性 | B0-B6 shadow 路径清楚，最可逆 | ordinal ID 会级联漂移；需双塔与新 ID 规则 | 影响面最大；所有塔消费者与形式桥重做 | 可把 greedy 设为候选，逻辑回退容易；存储/interface 迁移重 |
| 原文合法性 | 临时划分在 consumer Pending，Completed 冻结后最自然 | 延迟等待合法；用固定 N 冒充自然终结不合法 | 事实/解释分离自然；但必须给定唯一规则，不能永久多义 | provisional candidates 合法；多个候选同时冒充规范真值不合法 |
| 当前证据强度 | 最高：有 #58/#60/#61/#64 重放 | 仅有 #64 作为非贪心代理 | 无实现/重放，且定义欠完整 | #64 可作 selector 代理；无候选塔实现 |

## 5. A0：现状贪心塔 + 消费层组装

### 5.1 无 repaint 三门

- **前缀门：兼容。** 塔扫描是确定性左折叠，已产出窗口永不重访；`compose_level_resume` 只续扫尾部。源码证明口径见 `recursive_tower.rs:252-385`。#58 的完整 MoveView property 与 #63 的 Move+Unassigned 总 property 已给测试契约，但尚未生产接入。
- **entry 门：兼容。** WindowUnit 的出现不能直接当 Completed judge；盘整等后继，趋势等 A/C+MACD，订单从最晚证据下一 bar 才可用。
- **supersede 门：兼容。** C2 把版本复杂度留在消费层，塔 ID 可保持不变；但须增加 CompletedFreeze，防止同版本完成对象被普通 host 重组释放。

### 5.2 孤儿量

- 裸贪心塔 k1..3 raw gap：5,790，尾段 4。
- P0/P1 语义基线：目标 `A_completed=1,357 / A_pending=4,124 / U=313 / T=0`；全域 `U=2,028`。
- P2 当前只为 247 个已 Assigned 的 Trend-host 漏段提供潜在 Completed 转化，不能直接降低 U。
- #65 的 `F_ind` 对 A0 同样是下界。

### 5.3 增量复杂度与延迟

塔自身每级扫描单调前进，上层数量压缩，适合增量缓存；现有代码正依赖不可变窗口前缀消除 per-bar 全量重扫。组装器规范当前是无状态纯函数，单次需稳定排序、grouping、完整 chain 与延伸扫描；若每 bar 从头调用会重造 O(n²) 总成本，生产 adapter 应缓存派生索引，但缓存不能进入语义 interface。

判定延迟不是固定 N：seed 在第三个 lower settled 后可见；Completed consolidation 等下一异向组，Completed trend 等唯一 A/C 与 MACD；随后再加 1 bar 交易门。

### 5.4 Lean/formal 影响

A0 保留现有 `canonicalWindows/composeStep`、`MovesComposedFrom`、xs9 反退化和 Origin reanchor。代价是必须诚实地把这些定理解释为 WindowUnit 构造 soundness，而非 CompletedMove 完成性，并新增：

```text
Window material + lower ledger + as_of evidence
  -> CompletedMove/Attribution view
```

的纯函数、coverage totality、prefix 和 completion-freeze 证明。

### 5.5 迁移、可逆性与原文

#63 已给 B0→B6：先类型/reducer/interface，再 shadow 迁移区间套、higher center、背驰和买卖点；旧 WindowUnit 诊断路径可保留，故最可逆。锚：`unassigned-ledger-production-spec-20260713.md:624-656`。

原文上，贪心窗口只能叫稳定 seed，不得叫“最自然完成划分”。只要 consumer 允许 Pending/临时重组并冻结 Completed，A0 不违背第69/70课。A0 的弱点是当前 D2 策略没有独立版本名，容易把工程 seed 策略误当 C2 裁决的一部分。

## 6. A1：塔内在线非贪心划分（有界前瞻/滞后确认）

### 6.1 前缀稳定的充分条件

设 `Commit_t` 为截至 t 已提交的窗口前缀，`frontier_t` 为其右边界。A1 可满足前缀门，当且仅当至少满足：

```text
1. 每个选择只读取 judge_at≤t 的 lower/evidence；
2. Commit_t 是 Commit_T 的前缀（T>t），已提交部分永不重算；
3. frontier_t 之后只输出 Pending buffer，不冒充 final window；
4. 若允许改配，只能追加 partition supersede，历史 as_of=t 仍选择旧版；
5. 同 rule_version 的 Completed host 不因 partition supersede 改成员。
```

若窗口 `[i,i+2]` 延迟 N 个 lower 决定，则：

```text
judge_at(window) ≥ settled_at(lower[i+2+N])
entry_bar ≥ judge_at + 1
```

因此 A1 **可以**前缀稳定，但稳定来自 commit/version 规则，不是来自“N 有界”四个字。

### 6.2 固定 N 的反例边界

把所有合法三元组看成区间冲突图；相邻候选可形成任意长的冲突链。非贪心 selector 若做加权 interval scheduling，其最左选择可被链尾新增的一个更高优先级候选改变。对任何固定 N，都可构造长度大于 N 的交替冲突链，使全局最优的奇偶相位由 N 之外的尾部决定。

所以以下三者不能同时无条件成立：

```text
固定有限 N + 全局最优划分 + 已提交前缀永不变
```

除非另证该 center predicate/目标具有 N-locality。#64 只在 k1..3 三个截点各做 3 个 prefix assertion（9/9），这是实现样本，不是 locality 定理。事实锚：`orphan-reduction-paths-20260713.md:51-57,164-176`。

未版本化的 rolling DP 会直接构成反例：短前缀 t 选择 P0；未来 T 新候选使 argmax 变 P1；若塔覆盖 P0，则 `V(H,t)≠V(H',t)`。即使保留历史 as-of，若 P0 已标 Completed 而同版本 P1 改其成员，也违反 CompletedFreeze。

### 6.3 孤儿量

A1 没有实装重放。#64 全历史 DP 是最接近的非贪心代理：目标 `313→205`、全域 `2,028→2,001`，但目标回退 28、全域回退 524。它不是有界 N 算法，也没有无回退见证。

因此 A1 的诚实表述是：

- **[实测代理]** 某个全前缀 DP selector 可得 `U_target=205 / U_global=2,001`。
- **[推断]** 有界 N 结果未知，可能介于或差于 A0/DP；若强制 greedy fallback + 零回退，当前已证非零收益为 0。
- `F_ind` 仍不可消除。

### 6.4 复杂度、形式化与迁移

固定 N 缓冲可用 O(N) 内存；朴素每步 O(N)，固定宽度可优化到摊销 O(1)/O(log N)。代价是每一级额外 N 个 lower 延迟，而一个高层 lower 可跨很多 source bars，故必须同时报告 `lag_lower_count` 与 `lag_source_bars`。

形式化需替换现有“成立即 +3、永不回访”的核心引理，新增 buffer/frontier、commit prefix、选择 soundness 和全量/增量等价。现有 `ElementId=(level,ordinal)` 随前面改窗会级联漂移；A1 若版本化改配，ID 至少要纳入 `partition_policy_version + lower_ids/selection_version`，下游 persistent/coverage/interp 影响广。

双跑可用 `tower_scan_v1`/`tower_scan_v2`，但回滚不是简单切 flag：上层窗口、父子链和持仓 identity 都会级联变化，必须保留完整旧塔视图。

### 6.5 原文合法性

等待 N 段本身不违背“当下判定”，因为 judge_at 被诚实推迟；但“N 到了所以完成”没有原文依据。N 只能是等待更多划分证据的工程策略，不能替代走势终结。第70课更支持 Pending 后等自然选择，而不是固定时钟强制定窗。

## 7. A2：无窗口塔，塔只产 L0..Lk 线段账本

### 7.1 首要定义缺口

A2 看似最彻底消除贪心窗口身份，但“Lk 线段账本如何产生”尚未定义：

1. 若 Lk 仍由下级 CompletedMove/Center 递归产生，A2 实际把 C2 组装器变成下一层塔的前置，重新形成递归依赖；
2. 若 Lk 由独立线段算法产生，则它不再自动等于定义中的 `Move[k]` 构件，需新裁决证明两套级别递归等价；
3. 若所谓塔只存所有 settled lower 事实，窗口/走势都由查询生成，则 A2 本质是事件存储 + A4 consumer DP，而不是当前意义的递归塔。

在澄清前，A2 不是可直接实现的完整架构。

### 7.2 无 repaint 三门

- **前缀门：有条件强。** 若每层线段 ledger 有稳定 ID、`settled_at` 且只追加，所有窗口/走势缓存都可丢弃重算，则固定 as-of 纯函数天然可比较。
- 若 Lk ledger 会被未来 parser 结果回写，A2 仍会 repaint；“无窗口”不等于“事实不可变”。
- entry 与 supersede 门与 C2 相同；规则/segment migration 必须显式版本化。
- CompletedFreeze 仍必须存在。

### 7.3 孤儿量

A2 没有重放，不能宣称 U=0。#58 已经读取完整 lower ledger，故 A2 删除窗口塔不会凭空增加 lower 信息；它能改善的是 seed/partition 搜索空间，而这也能在 A4 consumer 内枚举。

诚实估计：

```text
U_target ∈ [F_ind_target, 313]   [推断]
U_global ∈ [F_ind_global, 2028]  [推断]
```

该区间不是性能承诺，只表示“不能突破 #65 划分无关下界，也没有证据证明优于当前 A0 上界”。

### 7.4 复杂度、形式化与迁移

ledger append 可很便宜，但窗口候选、非重叠选择、延伸、完成、higher center 都集中到查询。单级所有连续三元组候选是 O(n)，最佳非重叠选择可 O(n) DP；跨级递归与每 bar 重算若无索引会成为 O(k·n) 单查、O(n²) 全程。缓存只能是 adapter，不能改变 pure interface。

形式化影响最大：现有 `canonicalWindows/composeStep`、xs9 与 Origin recursive reanchor 不再是生产递归基础，只能降为某个缓存/候选生成器的 soundness 资产；需新建 settled segment ledger、CompletedMove assembly、higher-center recursion 和唯一分解桥。

迁移面覆盖所有直接消费 `LeveledMove`/ElementId/parent chain 的 classifier、coverage、interp、incremental 与 runner。可以双跑，但直到所有 higher-level identity 重建前不可切主。

### 7.5 原文合法性与模块深度

事实 ledger + as-of 视图与第69/70课相容；但选定规则后仍须唯一输出，不能用“多义性”逃避裁决。按模块 deletion test，删除 WindowUnit 塔后，窗口 soundness、候选冲突、ID、incremental frontier 全部在 consumer 重现；复杂度没有消失。除非 A2 证明窗口塔本身阻断了必要事实，现有证据不支持承担这次最大迁移。

## 8. A3：多候选划分塔（带版本，消费层择一）

### 8.1 前缀稳定的充分条件与反例

A3 可满足前缀门，但必须把“候选出生”和“择一”都当事件：

```text
Candidate { candidate_id, born_at, evidence, policy_version }
SelectionEvent { old_selection, new_selection, judge_at, rule_version }
```

查询 `as_of=t` 只能看到 `born_at/judge_at≤t` 的候选与 selection；未来择一只追加事件，旧选择不覆盖。entry 从 selection/完成最晚 judge_at 的下一 bar 开始；CompletedFreeze 禁止同版本重选已完成 chain。

反例：若 consumer 每次对“当前全部候选集”直接 `argmax`，而候选没有 born_at/selection ledger，则短前缀选 P0、未来新增 P1 后，历史 t 的查询也会选 P1，立即违反前缀等式。仅给 partition 一个 version 字段但查询自动取 latest，同样失败。

### 8.2 孤儿量

候选集本身不决定 U，selector 才决定：

- 选择 greedy candidate：实测参考 `U_target=313 / U_global=2,028`。
- 选择 #64 DP candidate：实测参考 `U_target=205 / U_global=2,001`，伴随 28/524 回退。
- 选择“零回退优先” candidate：当前无非零收益见证，生产安全下界为 0。
- 任一 selector 仍受 `F_ind` 下界约束。

因此不能把 A3 写成“塔产候选所以孤儿更少”；必须把计数归属到 `(candidate_generator_version, selector_version)`。

### 8.3 复杂度与延迟

合法窗口数可线性枚举，但合法 partition 集最坏指数增长。若塔显式物化全部 partition，存储和事件不可控；若压成 interval-DAG，节点/边可近线性，而 consumer 用 DP 择一路径——这已接近 A4，即塔只提供候选事实图，策略仍在 consumer。

候选可在第三 lower settled 后出现，但法定选中可能因未来更自然候选而长期 provisional。若为了低延迟每次重选，会放大 supersede/churn；若等唯一选择，延迟可能无固定上界。

### 8.4 Lean/formal 与迁移

单候选窗口的重叠、三 witness、compose/descend soundness 可复用。需要新增：候选生成完备性、冲突图非重叠、selector 决定性、选择事件前缀稳定、同版本 CompletedFreeze。原 `canonicalWindows` 的“唯一规范窗口族”不再直接是生产结论。

迁移需新 candidate/partition/selection ID 与账本，塔 interface 从小型 WindowUnit 序列扩为候选图。回滚可选择 greedy path，因此逻辑可逆性好；但 interface、存储和测试面远大于 A0/A4。

### 8.5 原文合法性

第70课支持临时多义观察和后续自然选择，因此“候选”作为 provisional 事实合法；第38/101/102课又要求选定规则后的唯一分解，所以多个候选不能同时作为规范 Completed 真值。A3 必须明确：候选不是走势完成事实，只有版本化 selection 后的 view 才能供消费者使用。

## 9. A4（建议补充）：C2 + 消费层版本化 PartitionPolicy

A4 保持 A0 的 W/V 双轨，但把窗口选择从隐含实现细节提升为消费层纯策略：

```text
Immutable material:
  lower ledger + greedy WindowUnit seeds + all legal local seed facts

Pure view:
  assemble_level_view(as_of, rule_version, partition_policy_version)
```

优先在 consumer 枚举替代三元组/冲突图，并以 `regressions=0` 为硬门选择；塔的 greedy WindowUnit 继续作为 `greedy_v1` 稳定 seed/cache，不被宣称为唯一自然划分。

优点：

1. 直接检验 #64 指向的 D2 问题，不动 C2 seam、ElementId 与 Lean 窗口资产；
2. 同一 lower ledger 下可 shadow 比较 greedy/DP，旧版本随时可回退；
3. 若候选图/DP 证明有稳定非零收益，再决定是否因性能把内部缓存下沉到塔；下沉是实现优化，不先变语义 seam；
4. `PartitionPolicyVersion`、direction provider、A/C provider 分开，计数不再混因。

当前 A4 不能直接生产化 #64 selector，因为它仍有 524 个全域回退；下一重放须使用 #64 已建议的目标优先级：零回退 → 最大化全域净减少 → 最小 supersede → 最大保留原窗 → 最后才最大 gap coverage。锚：`orphan-reduction-paths-20260713.md:218-225,335-346`。

## 10. 原文合法性总评

| 原文要求 | A0 | A1 | A2 | A3 |
|---|---|---|---|---|
| 当下临时判断可修正 | consumer Pending/supersede 可承载 | buffer/Pending 可承载 | pure as-of view 可承载 | candidate/selection 可承载 |
| 已完成图形不可修改 | 需补 CompletedFreeze | commit 不能把未终结对象伪 Completed | 需补 CompletedFreeze | selection 不得重写同版本 Completed |
| 等走势自然选择，不预测 | 不把 greedy seed 当完成即可 | N 只能是延迟，不是终结规则 | selector 仍需定义证据 | 候选可并存，但法定选择须等证据 |
| 多义性不是含糊性 | rule/version 下唯一 view | commit policy 下唯一 | pure rule 下唯一 | selector/version 下唯一 |
| 完整走势由完成次级走势构成 | consumer CompletedMove 承载 | 塔若直接递归 provisional window 有风险 | 可直接建模，但成本最大 | 只有 selected+Completed 可递归 |

结论：A1/A3 的前瞻或多候选并不天然违背“当下判定”；真正违法的是把未来形成的选择回填成当时已知，或把 provisional 选择标成 Completed 后再改。A0 也必须接受同一约束。

## 11. 建议给 escalate 的补充裁决范围

以下是建议材料，不是裁决文本：

1. **维持 C2 seam。** WindowUnit/稳定 lower facts 与 CompletedMove/Attribution 的职责分层不因 #64 自动重开。
2. **扫描策略独立版本化。** `greedy_left_to_right_v1` 是一种 `PartitionPolicy`/seed policy，不是 C2 的定义组成；任何替代策略必须单独 rule/version、计数和 shadow replay。
3. **consumer-first。** 能由同一不可变 lower ledger 纯计算的替代划分，先在 consumer A4 实现；只有性能证据证明必须下沉时，才评估改塔，不把缓存位置升级成语义裁决。
4. **补 CompletedFreeze。** 同 rule version 的 Completed chain/membership 不得被 ordinary supersede；规则迁移必须保留旧版本并显式 reopen/new generation。
5. **A/U/T 与 completion 正交。** 改 partition 必须同时报告 Assigned、Completed、released、net，不能只报 old-U resolved 或 seed coverage。

## 12. 量化触发判据

以下阈值均为**治理建议**，用于决定何时回到 `escalate`，不是从原文推导的定义数。

### 12.1 触发“扫描策略补充裁决”

P0-P2 全部落地、direction/A-C/rule version 冻结后，在至少 3 个互不重叠数据分区同时满足：

```text
X1: nonterminal_U_global / VisibleLower > 1.0%
    且 nonterminal_U_global > 100

Y1: #65 strategy_related_U / nonterminal_U_global > 50%
```

其中 `nonterminal_U` 排除合法尾部 Pending，不把 Tombstoned 当待解 U。按当前 BTC 预览，`2,028/55,624=3.645908%` 已超过 X1，但 P2/#65 未完成，故现在不能宣布触发完成。

### 12.2 允许替代 PartitionPolicy 从实验进入候选生产

同一冻结 fixture 上必须同时满足：

```text
P1: regressed_to_unassigned == 0
P2: completed_to_pending_or_unassigned == 0
P3: global_U_net_reduction ≥ 25%
P4: 所有 commit/selection 边界通过 prefix equality，而非仅 9 个采样点
P5: 旧事件/旧 Completed bytes 与历史 as_of 结果不变
```

以当前 `U_global=2,028` 为参照，P3 意味着至少净减 `507`。#64 当前只净减 27 且回退 524，明显不达门。

### 12.3 触发“修订 C2 seam”而非只补扫描策略

只有在替代 policy 已通过 §12.2 后，再同时满足：

```text
C1: ≥80% 的已证净收益无法由同一 immutable lower ledger 上的
    consumer pure function/A4 重现；必须依赖塔内新增的在线状态或证据。

C2: A1/A3 原型在完整 prefix property、entry+1、append-only、
    CompletedFreeze 全过的前提下，p99 增量耗时 ≤ 2×A0，
    内存 ≤ 4×A0；若用 A1，额外确认延迟 N ≤ 3 个 lower，
    并单列其 source-bar p50/p95/p99。

C3: Lean 迁移计划能保留或替代窗口 soundness、ID 稳定、
    incremental equivalence 与 recursive component 证明，无未标注语义断层。
```

高 U 或“某些孤儿可换划分获救”本身只触发 D2 复审；只有 C1 证明现有 seam 阻断必要信息，才触发修订 C2。

## 13. 必要重放与可复现性

本任务复用了 #64 已编译的隔离二进制，只写 `/tmp`：

```bash
/private/tmp/c58-assembler-work/rust/target/release/task64_orphan_paths \
  > /tmp/task66-task64-replay.log \
  2> /tmp/task66-task64-replay.err

cmp /tmp/task66-task64-replay.log /tmp/task64-orphan-paths.log
shasum -a 256 /tmp/task66-task64-replay.log
```

结果：

```text
cmp: match
sha256: a272f35ff4259f9a9db56be777a58675635011888406807cb32c701186a1919b
stderr: 0 lines
TARGET: 313 -> 205; resolved=136; regressed=28; net=108
GLOBAL: 2028 -> 2001; resolved=551; regressed=524; net=27
PREFIX_GUARDS: 9/9 pass
```

这次重放确认 #64 数字可重复；它不补足 #65，也不把 9 个 prefix 样点升级为形式证明。

## 14. 最终回答

1. **C2 与贪心扫描是独立决策。** raw 孤儿主要由贪心 seed policy 产生；幽灵状态由缺完整 consumer/ledger 产生；P0-P2 后残余则由 partition、grouping、direction、completion 与划分无关失败共同决定。
2. **A1 可前缀稳定，但固定 N 不足以证明。** 必须有不可回退 commit frontier 或版本化 partition supersede；全局最优存在任意长冲突依赖，不能同时保证固定 N 与永不改前缀。
3. **A3 可前缀稳定，但择一也必须是事件。** 未按 as-of 版本化的 current-argmax 会立即 repaint；候选不是 Completed 真值。
4. **A0 的 P0/P1 实测残余是目标 313、全域 2,028。** P2 当前只承诺把部分 Assigned-Pending 变 Completed，不承诺降低 U；落地后必须重跑。
5. **#64 对替代划分给出非零毛收益，但没有生产安全收益。** 目标净减 108，却在全域只净减 27并释放 524；零回退门下已证非零收益为 0。
6. **A2/A3 当前没有足够证据优于 A0/A4。** A2 定义与证明迁移最大；A3 若压缩成候选 DAG，选择复杂度仍回到 consumer。
7. **建议维持 C2、补充裁决扫描策略独立版本化和 CompletedFreeze。** 先做 A4 无回退重放；只有达到 §12.3，才重开 C2 seam。
