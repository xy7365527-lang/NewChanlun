# 孤儿段消解路径专项调研（task #64）

- 日期：2026-07-13
- 性质：只读调研 + 隔离重放；不修改生产 Rust、塔、Lean、定义或原文
- 数据：BTC 1m 全历史 `4,613,599` bars，末端 `as_of=4,613,598`，L0 `40,003` 线段
- 法定前提：C2——塔只产不可变 `WindowUnit`，完成走势由消费层 as-of 纯函数组装；Q5 采 C 追加式 `Unassigned` 账本。锚：`chanlun/escalate/c-ruling-decision-20260713.md:11-19,21-24`
- 原型边界：#58 尚无生产调用方；本次只在 `/private/tmp/task64-orphan-replay` 复制原型并重放。锚：`chanlun/review-results/assembler-spec-20260712.md:5-9`

## 0. 结论先行

### 0.1 口径

以下三种数不得混名：

1. **塔漏段数**：k1..3 的失败 `i += 1` 对象 `5,790`，另有尾部不足三元组 `4`；这是 #61 成因分类的对象域。锚：`chanlun/review-results/q5-orphan-attribution-research-20260713.md:6,11-15,37-46`
2. **Assigned 数**：lower 已唯一进入某个组装 host；host 可以是 `Pending`，不得冒充 Completed。锚：`chanlun/review-results/q5-orphan-attribution-research-20260713.md:121-133`、`chanlun/review-results/unassigned-ledger-production-spec-20260713.md:251-275`
3. **可消费数**：只有 `Assigned && host.completion==Completed` 可进入区间套；Assigned-to-Pending 仍须返回 coverage gap。锚：`chanlun/review-results/unassigned-ledger-production-spec-20260713.md:406-426`

因此，“吸收”在本文分别报告 `Assigned(any host)` 与 `Assigned+Completed`；不同机制存在重叠，不相加。

### 0.2 全历史主结果

| 机制 | 可处理成因桶 | 实测 Assigned / 结构上界 | 实测 Completed / 可消费 | 对当前未归属的净减少 | 结论 |
|---|---|---:|---:|---:|---|
| 中枢延伸 | 跨窗界、exact-three 非种子、尾段 | `2,644`（失败漏段 `2,640` + 尾段 `4`） | `674` | 相对裸塔可减少；相对 #58 当前原型增量为 `0`，因其已实现 | 第一优先生产化；不动塔 |
| 趋势型组装 | 跨窗界、exact-three 非种子 | `247` 个漏段进入 `139` 个结构 Trend host（k1..3） | `0` | Assigned 层已吸收；Completed 层仍为 `0` | 缺唯一 A/C hook，不得伪完成 |
| 跨窗重组 DP | 主要是跨窗界；链边界也可带入 exact-three 非种子 | `4,457` 个局部候选全获某个非重叠种子覆盖；目标残余 `313→205` | 目标旧未归属中 `29` 转入 Completed host | 目标域 `-108`；全 lower 域仅 `-27` | 有实测下界，但伴随 `524` 个回退为未归属，不能按当前目标直接生产化 |
| #53 入口放宽 | 不是归属机制 | `202` 个新增几何入口事件 | 未重跑，不能沿用 `213` | 直接 `0` | 只扩候选入口，不制造 host，也不应制造新孤儿 |
| 在线尾段 | `TERMINAL_INSUFFICIENT` 4 例 | 当前均进入 Pending host | `0` | `4` 已有待定归属 | 用既有 as-of/supersede 结算，无需新机制 |

重放原数：`/tmp/task64-orphan-paths.log:1-7`。#60 已证明 sequence-envelope 下 k1..3 原始残余为 `262+45+6=313`，且该数不是理论永久不可归属：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:127-137`。

### 0.3 建议裁定

1. **先生产化 #58 的完整 lower-ledger chain + 中枢延伸 + C 账本 seam**；这是已有定义锚、无方向新增裁决、无塔改动的最大确定收益。
2. **趋势补全先裁 direction provider 与唯一 A/C 配对，再接 MACD 真计算**；当前能证明 `247` 个漏段已有 Pending 归属，不能证明其中多少可 Completed。
3. **跨窗界重组保留为实验路线**。本次给出一个可复现的非零下界，但其“覆盖全部 4,457”是以替换 `3,731` 个原窗口为代价；必须先加“不得新增未归属 / supersede 成本”约束，不能只最大化 gap coverage。
4. **#53 不计入孤儿消解量**，直到它的事件与 `MoveView` host membership 做 as-of join 并重放。

## 1. 法律与工程硬边界

### 1.1 原文合法性

- 走势类型至少由三段次级别走势构成；盘整达到最小结构后可以继续围绕同一中枢延伸；趋势形成两个依次同向中枢后也可以继续延伸。锚：`docs/chanlun/text/blog/018-第18课.md:34-44`
- 当下图形未完成时允许待定；已经完成的图形不能被以后数据修改。锚：`docs/chanlun/text/blog/069-第69课.md:24-28`
- 多样性分解允许暂定最有利观察划分，但应“等走势走出最自然的选择”后再作更合理划分，不能预先强吸。锚：`docs/chanlun/text/blog/070-第70课.md:16-20,24-40`
- 只有属于同一特征序列的元素才可讨论包含；中间地带可以保持待定，不能跨对象域强行归类。锚：`docs/chanlun/text/blog/071-第71课.md:18-28`

这四组原文共同排除两种捷径：把任意相邻 lower 强塞进旧 host，以及用全历史最优分窗回写旧 as-of。

### 1.2 C2 与三道无 repaint 防线

- 塔窗口命中取三段、失败前进一段；中枢延伸状态在塔扫描中不存在，趋势多中枢组装也不存在。锚：`chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:24-33,60-72`
- 组装器必须只读截至 `as_of=t` 的可见材料，并满足短前缀与未来超集在固定 t 下全字段相等。锚：`chanlun/review-results/assembler-spec-20260712.md:54-73`
- 强制三门是：前缀稳定 property、`entry_bar >= judge_at+1`、追加 supersede 而不覆盖旧版本。锚：`chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:232-236`

本次跨窗原型在 k1..3 各跑 3 个截点，共 9 个 `full-input@t == actual-prefix@t` 全字段断言，全部通过：`/tmp/task64-orphan-paths.log:2-7`。这只验证原型的前缀实现，不把其实验 direction/分窗目标升级为定义裁决。

## 2. 基线：成因桶与当前组装归属

### 2.1 #61 成因桶

| target k | 跨窗界可重组 | exact-three 真不可组合 | 尾段在线待定 |
|---:|---:|---:|---:|
| 1 | 3,039 | 767 | 2 |
| 2 | 1,044 | 353 | 0 |
| 3 | 374 | 213 | 2 |
| 合计 | **4,457** | **1,333** | **4** |

事实锚：`chanlun/review-results/q5-orphan-attribution-research-20260713.md:21-31,37-46`。`4,457` 只是一段能进入某个包含自身合法三元组的局部上界；#61 已明示重叠候选会冲突，且完整走势还要满足延伸、同向中枢、终结和 as-of：同文件 `:46`。

### 2.2 #58 sequence-envelope 基线重放

本次用 #60 最近的 sequence-envelope 适配复核 k1..3：

| 原因 | Completed consolidation | Pending trend | 其他 Pending | 无 host | 合计 |
|---|---:|---:|---:|---:|---:|
| 跨窗界 | 1,048 | 144 | 3,004 | 261 | 4,457 |
| exact-three 真不可组合 | 309 | 103 | 869 | 52 | 1,333 |
| 尾段 | 0 | 0 | 4 | 0 | 4 |
| 合计 | **1,357** | **247** | **3,877** | **313** | **5,794** |

原数逐级见 `/tmp/task64-orphan-paths.log:2,4,6`。其中尾段 k3 的 2 例为 Pending consolidation，日志为便于汇总列入 Pending；其余尾段为 Pending undetermined。#60 的独立基线给出失败漏段 `5,477` 归入任一 host、`313` 无 host：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:127-137`。

**关键边界：**“exact-three 真不可组合”只否定把该 lower 当作三段 seed 的可能，不否定它按坐标进入别的 Completed/Pending host chain；本次 1,333 中已有 1,281 获得 host，只有 52 无 host。这正是墓碑不能由 seed failure 单独触发的原因。锚：`chanlun/review-results/q5-orphan-attribution-research-20260713.md:135-145`。

## 3. 机制一：消费层中枢延伸吸收

### 3.1 合法性与实现路径

原文允许盘整和趋势在最小结构后延伸：`docs/chanlun/text/blog/018-第18课.md:38-44`。#58 的纯函数判据是：后续 lower 与种子核心区间重叠，即 `high(s)>=ZD(z) && low(s)<=ZG(z)`，累积 `GG/DD/end`，首个不重叠 lower 终止，并在下一种子/下一走势前截断：`chanlun/review-results/assembler-spec-20260712.md:108-121`。

合法生产路径：

1. 塔仍只输出不可变 seed，不改 `compose_level`。
2. `MoveView(as_of=t)` 从完整 lower ledger 取料；仅扫描 `end<=t` 的 component。
3. 延伸 lower 唯一落入 host chain 后即可 Assigned；host Pending/Completed 另记。
4. 后来出现 break、下一 seed 或后继走势时，只追加新 Move version；若 membership 改变，与 C 账本原子 supersede。

该路径天然符合 C2 双轨职责：`chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:216-230`。

### 3.2 实测吸收量

| 桶 | Assigned(any host) | 其中 host Completed | Pending |
|---|---:|---:|---:|
| 跨窗界 | 2,581 | 662 | 1,919 |
| exact-three 真不可组合 | 59 | 12 | 47 |
| 尾段在线待定 | 4 | 0 | 4 |
| 合计 | **2,644** | **674** | **1,970** |

逐级原数：`/tmp/task64-orphan-paths.log:2,4,6`。

- **下界（当前原型）**：2,644 个对象的 host center 明确把其坐标列入 `absorbed`；这是 Assigned 归属证据。
- **Completed 下界**：674 已在 Completed consolidation host 中，可按 C2 进入消费者。
- **相对生产的增量 [推断]**：若生产当前完全没有 #58 seam，生产化该机制可取得上述量级；若比较对象就是 #58 原型，则增量为 0，因为数字已包含在基线。

### 3.3 与 C 账本衔接

- 原先 `OPENED/REOPENED` 且新 as-of 首次出现唯一延伸 host：追加 `ASSIGNED(reason=LEGAL_HOST_ESTABLISHED)`；同 correlation 追加 `MOVE_CREATED`，或 `MOVE_SUPERSEDE(reason=ADOPT_ORPHAN)`。锚：`chanlun/review-results/unassigned-ledger-production-spec-20260713.md:264-273,285-296`
- 若 lower 从一个旧 host 改配新 host：`ASSIGNED(old)->ASSIGNED(new)`，reason 为 `HOST_SUPERSEDED_ADOPTED`，Move 侧同批写 `ADOPTED`。同文件 `:271-273`。
- 仅 host 从 Pending 变 Completed、membership 未变时，不制造第二次 `ORPHAN_ASSIGNED`；只追加 host 完成版本，coverage 从 `AssignedToPendingHost` 变为可消费。

### 3.4 风险与反例

1. 仅与外围 `GG/DD` 相交不够；合法吸收判据是与固定核心 `[ZD,ZG]` 相交。扩大为“价格靠近”会变成方案 B 强吸。
2. 不能越过下一中枢种子或下一走势组；否则一个早期 seed 会吞掉后续独立走势。
3. 2,644 与趋势机制重叠 79 个对象（跨窗界 75、真不可组合 4），不可相加。原数：`/tmp/task64-orphan-paths.log:2,4,6`。
4. 674 Completed 不是“延伸本身证明完成”；完成证据仍来自后继异向走势，延伸只证明 membership。

## 4. 机制二：趋势型组装补全（C-c）

### 4.1 合法性与缺口

趋势至少含两个依次同向中枢；塔单元固定只有一个中枢，因此缺口属于消费层组装而非塔修复。锚：`chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:66-72`、`docs/chanlun/text/blog/018-第18课.md:42-44`。

#58 的结构判据为同方向 maximal window run，且相邻延伸后中枢满足：上涨 `DD(next)>GG(prev)`，下跌 `GG(next)<DD(prev)`；趋势完成还必须有显式 A/C pair、source→MACD 坐标映射和真实 `area(C)<area(A)`。锚：`chanlun/review-results/assembler-spec-20260712.md:77-86,123-154`。

### 4.2 实测量

- k1..3 有结构 Trend host `84+40+15=139` 个；独立 #60 原数：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:139-154`。
- 这些 Trend host 包含原塔漏段 247 个：跨窗界 144、exact-three 真不可组合 103；本次逐级重放：`/tmp/task64-orphan-paths.log:2,4,6`。
- Completed Trend 为 **0**，原因不是“BTC 没有趋势”，而是原型明确未提供 A/C hook；缺 hook 时必须 `Pending(MissingDivergencePair)`。锚：`chanlun/review-results/assembler-spec-20260712.md:156-164`。

因此：

- **Assigned 下界**：247 已有 Pending Trend host。
- **Completed 下界**：0。
- **Completed 条件上界 [推断]**：247 个漏段都可能随其 139 个 host 中一部分合法完成而变得可消费；不能假定 247 全部完成，也不能把 #54 的 `38` 或 `0/213` 代入。#60 已指出对象类型不同：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:82-88`。

### 4.3 实现路径

1. 先裁并版本化 `direction_provider`；#58 禁止把首项占位方向冒充定义裁决：`chanlun/review-results/assembler-spec-20260712.md:52`。
2. 以完整 lower chain 生成同向窗口组和延伸后中枢序列。
3. 单独裁唯一 A/C pairing：A/C 必须是该 host chain 中当时可见、已完成的可比段；配对结果显式传给 hook。
4. 复用真实 MACD 映射/面积函数；映射缺失、未背驰或 pair 未出现均保持 Pending。
5. `judge_at` 取 C 端、MACD 坐标和 host 终结证据中最晚可知点；任何消费者只可从下一 bar 使用。

### 4.4 C 账本与风险

- 只要唯一 Pending Trend host 已建立，就可 `ORPHAN_ASSIGNED`；不必等趋势完成。趋势完成事件只改变消费资格。
- 若新 A/C 证据导致 host 边界重组，必须 `MOVE_SUPERSEDE + ADOPTED/RELEASED`，被释放对象写 `REOPENED`，不能删旧 assignment。锚：`chanlun/review-results/unassigned-ledger-production-spec-20260713.md:267-277`。
- 反例：自动取“最强背驰 pair”会读取未来并改变历史 A/C；自动取任意首尾段会伪造可比性。二者都违反第 70 课的自然选择与 as-of 前缀门。

## 5. 机制三：跨窗界重组

### 5.1 原型与完整判据

本次原型对每级执行：

1. 枚举每个 lower 的所有包含自身三元组，仍调用该层真实 center seed predicate。
2. 动态规划选择互不重叠窗口；目标按字典序最大化 `(覆盖 gap 数, 保留原窗口数, 总窗口数)`。
3. 用与 sequence-envelope 同源的外缘方向适配，把选择结果送入未改的 #58 `assemble_move_view`。
4. 只把存在唯一 host 的对象记 Assigned；只把 #58 判为 Completed 的 host 记可消费。
5. 固定 1/4、1/2、3/4 三个截点逐级做未来超集与实际前缀全字段相等断言。

这满足本实验的 seed、非重叠、完整 lower chain、延伸、同向中枢、终结和 as-of 判据；但 direction provider 与 DP 优化目标仍是 **[推断/实验]**，不是新裁决。

### 5.2 4,457 局部上界的可行子集

| k | 局部上界 | 新选窗口 | 保留原窗口 | seed 覆盖 | 进入 Completed host | 其中旧 gap-unassigned 且在新 seed 内 |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 3,039 | 11,934 | 9,479 | 3,039 | 721 | 20 |
| 2 | 1,044 | 3,513 | 2,689 | 1,044 | 229 | 2 |
| 3 | 374 | 984 | 711 | 374 | 87 | 0 |
| 合计 | **4,457** | **16,431** | **12,879** | **4,457** | **1,037** | **22** |

原数：`/tmp/task64-orphan-paths.log:3,5,7`。

**结果解释：**4,457 个局部候选可以同时被某个互不重叠选择覆盖，故“候选冲突必然使可行子集小于 4,457”被本数据/目标否证；但该选择只保留 k1..3 原 `16,610` 个窗口中的 `12,879` 个，替换 `3,731` 个。`1,037` 也不是新增消解量，其中大部分原本已有 host。

### 5.3 净效应：必须同时报回退

| 口径 | 原未归属 | 重组后未归属 | 旧未归属→有 host | 其中→Completed | 旧有 host→未归属 | 净减少 |
|---|---:|---:|---:|---:|---:|---:|
| #61 目标桶（失败漏段+尾段） | 313 | 205 | 136 | 29 | 28 | **108** |
| 全部 k1..3 lower | 2,028 | 2,001 | 551 | 182 | 524 | **27** |

逐级原数与成因拆分：`/tmp/task64-orphan-paths.log:3,5,7`。

重组后的 #61 目标残余为：跨窗界 `170`（143+24+3），exact-three 非种子 `35`（25+9+1），尾段 `0`。这里 exact-three 对象能减少，是因为新组边界令其按坐标进入 host chain，不是把它伪造为 seed。

- **实测毛下界**：至少有 136 个目标旧未归属得到 host，其中 29 得到 Completed host。
- **实测目标域净下界**：在该可复现选择下，目标残余净少 108。
- **实测全域净下界**：全 lower 完全分类只净少 27；这是“真正减少对象总数”应采用的保守下界。
- **生产安全下界 [推断]**：若验收要求 `regressed_to_unassigned==0`，本原型尚未给出非零见证，当前只能报 0；需要稳定性约束后的新优化重放。

### 5.4 C 账本衔接

该机制不是批量覆盖旧结果，而是一组原子版本事件：

1. 136 个旧未归属被新 host 采用：`ASSIGNED(LEGAL_HOST_ESTABLISHED)` + `MOVE_SUPERSEDE(reason=ADOPT_ORPHAN)` 或 `MOVE_CREATED`。
2. 28 个目标桶及全域共 524 个旧有 host 对象被释放：同 correlation 写 `MOVE_SUPERSEDE(RELEASED)` + `REOPENED(HOST_SUPERSEDED_RELEASED)`。
3. host 未 Completed 的 assignment 进入 `AssignedToPendingHost` coverage gap，不得直接进入区间套。
4. 所有事件 `judge_at` 只能取新划分在该前缀可知的时点；旧 as-of 仍解析旧 Move version。

原子批次和状态转移硬约束：`chanlun/review-results/unassigned-ledger-production-spec-20260713.md:232-243,264-277,350-374`。

### 5.5 风险与反例

1. 最大化 gap coverage 会诱导大规模窗口替换；“4,457 全覆盖”不等于最小 churn，更不等于全域净最优。
2. k3 全域未归属由 67 增至 77，说明逐级局部最优可伤害完全分类；验收必须 per-level 和 global 同时过。
3. 原型 direction 只是 #60 敏感性适配；方向裁定变化会改变 maximal run、Completed 数和边界归属。
4. 未来全历史最优 DP 不能回写过去；生产只能 append 以当时前缀决定的新版本。
5. 建议下一版优化目标按优先级改为：`regressions=0` 硬门 → 最大化全域净减少 → 最小 supersede 数 → 最大化原窗口保留 → 才比较 gap seed coverage。

## 6. #53 入口放宽 202 例的连带效应

### 6.1 最新证据与谱系缺口

最新保留的 #53 报告位于 `/private/tmp/p0-replay-work2/chanlun/review-results/p53-entry-relaxation-20260712.md`，记录：旧入口 `E_old=31`，几何上界/新入口 `U=E_new=213`，故相对旧集合新增 `202`；来源为旧 true 11、旧 false 9、无旧同 CpId 193。锚：该文件 `:1-10,14-37,89-105`。

当前主工作树 `chanlun/review-results` 没有该报告，`.chanlun/goals/events.jsonl` 也没有 2026-07-12 的该 P53 事件；日志最后一行仍为 2026-07-01，锚：`.chanlun/goals/events.jsonl:252`。其中 `:44-48` 的“#53”是 2026-06-27 递归塔升级同号旧任务，不是本次 202 例入口放宽。故本文把 `/private/tmp` 报告称为“最新保留实装证据”，不冒充当前主树已登记/已合并事实。

### 6.2 会新增归属还是新增孤儿

**直接效应：两者都不新增。** `CandDeltaEntryEvent` 是几何候选入口，不是 lower membership、Move host、Unassigned event 或完成证据。报告也明示未授权接入 strict-chain、交易、订单或历史入场：`/private/tmp/p0-replay-work2/chanlun/review-results/p53-entry-relaxation-20260712.md:258-268`。

若后续显式把 202 个事件接到 C2：

1. 先以事件自身 `judge_at` 查询同一 `MoveView(as_of,rule_version)`。
2. 只有唯一 host membership 成立才改变归属；否则事件保持候选，不创建第二份 orphan 身份。
3. 若 host Pending，记 Assigned-to-Pending coverage；若 Completed，才可进入 strict consumer。
4. 不得给 193 个无旧事件对象伪造 `a_interval/divergence_confirm_src`；报告原证据：同文件 `:39-45,107-125`。

#60 对全部 213 个 SUCCESS 的最近映射是 CompletedDistinct 22、至少一侧 Pending 146、至少一侧无归属 45：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:73-88`。由于 202 与这三桶没有逐对象 join，不能直接相减；仅能给出 **[推断]** 范围：新增 202 中 CompletedDistinct 为 11..22、Pending 为 135..146、至少一侧无归属为 34..45。它们表示“候选面对现有 coverage 的状态”，不是新增 34..45 个孤儿；那些 gap 已属于 lower 完全分类。

## 7. 尾段在线待定 4 例

### 7.1 当前状态

#61 将尾部不足三个 lower 的 4 例与失败 `i+=1` 明确分开：`chanlun/review-results/q5-orphan-attribution-research-20260713.md:21-31,37-42`。本次 #58 重放中四例都进入唯一 Pending host，并都命中中枢延伸：k1 两例 Pending undetermined，k3 两例 Pending consolidation；原数：`/tmp/task64-orphan-paths.log:2,6`。

因此在 C 账本投影里，它们应是 `AssignedToPendingHost`，而不是因为“末端”自动 tombstone。

### 7.2 结算路径

1. 新 lower settled 后，以新 `as_of` 重算同一纯视图。
2. 若仍属同一 Pending host：仅追加 host 版本/延伸证据。
3. 若后继异向走势完成盘整：host 变 Completed，membership 不变，不重复 ORPHAN_ASSIGNED。
4. 若形成新唯一 host：按 `LEGAL_HOST_ESTABLISHED + ADOPT_ORPHAN` 转正。
5. 若旧 host supersede 后释放：追加 `REOPENED`，等待新 host。
6. 开放流永不因“当前没有更多 bar”触发墓碑；必须有显式 `CoordinateFinalized`。

这完全落在既有状态机与 as-of 规则内，无需新机制：`chanlun/review-results/unassigned-ledger-production-spec-20260713.md:264-277,366-374,510-534`。

## 8. 剩余真不可组合与预期终态

### 8.1 当前终态快照

不用跨窗实验时，sequence-envelope 下 #61 目标域是：

| 状态 | 数量 | 说明 |
|---|---:|---|
| Assigned + Completed | 1,357 | 可消费 |
| Assigned + Pending | 4,124 | 含尾段 4；只返回 coverage gap |
| Unassigned | 313 | 跨窗界 261 + exact-three 非种子 52 |
| Tombstoned | 0 | 当前没有显式最终化证据 |

采用本次跨窗实验选择后，目标域 Unassigned 为 205：跨窗界 170、exact-three 非种子 35，尾段 0；但全域只净减 27 且发生 524 次释放，故不建议把 205 写成生产承诺。

### 8.2 长期挂起与墓碑分布

- **在线开放域的预期 [推断]**：剩余 205（若不采用实验则 313）继续为 Unassigned/REOPENED，`ORPHAN_TOMBSTONED=0`。35 个 exact-three 非种子对象是墓碑风险最高的一组，但本次仍有 19 个原真不可组合未归属对象被重组 host 吸收、其中 7 个进入 Completed，证明“无 seed”不能直接墓碑。原数：`/tmp/task64-orphan-paths.log:3,5,7`。
- **显式最终化域的上界 [推断]**：在同一 rule version 下，剩余每个对象最终必须二选一：后继证据建立 host 后 Assigned，或通过 T1..T5 后 Tombstoned。故 Tombstone 数在 `0..205`（基线方案为 `0..313`），没有证据支持更窄点估计。
- **禁止的推断**：不能把当前全历史末端当 `CoordinateFinalized`，不能把 35/52 个 seed failure 直接写成墓碑，也不能在有 PendingMove 跨越时墓碑。

墓碑五项合取门是 lower settled、后继封闭、显式坐标最终化、穷举无合法 host 且无 Pending 跨越、`judge_at` 精确取四时点最大值：`chanlun/review-results/unassigned-ledger-production-spec-20260713.md:500-538`。

### 8.3 触发 `ADOPT_ORPHAN` 的事件清单

注意：持久化 Unassigned 侧写 `ASSIGNED`；`ADOPT_ORPHAN` 是同 correlation 的 Move supersede reason，不是另一种 Unassigned 状态枚举。锚：`chanlun/review-results/q5-orphan-attribution-research-20260713.md:121-133`、`chanlun/review-results/unassigned-ledger-production-spec-20260713.md:245-275`。

| 可见新事实 | 最早 judge_at | Unassigned 侧 | Move 侧 | 是否立即可消费 |
|---|---|---|---|---|
| 中枢延伸首次令 lower 唯一进入 host | overlap lower settled 且 host membership 可判时 | `ASSIGNED / LEGAL_HOST_ESTABLISHED` | `MOVE_CREATED` 或 `MOVE_SUPERSEDE(ADOPT_ORPHAN)` | 仅 host Completed 时 |
| 新窗口/跨窗选择首次建立唯一坐标 chain | 该选择全部 seed 与方向材料可见时 | 同上 | supersede 必列 adopted lower IDs | 同上 |
| 趋势同向第二中枢令 Pending host 建立 | 第二中枢及方向证据可见时 | 同上 | create/supersede | 否，仍 Pending |
| A/C + MACD 令既有趋势完成 | C 与 MACD 映射最晚可知时 | 若此前已 Assigned，不新增 assignment | host completion supersede | 是，从下一 bar |
| 在线尾段新增 lower 后首次建立唯一 host | 新 lower settled 时 | 同上 | create/supersede | 视 completion |
| #53 候选经显式 join 后确实改变 host membership | entry event 与 host 证据最晚时点 | 同上；候选事件本身不触发 | create/supersede | 视 completion |
| host 重组仍采用已 Assigned lower | 新 host 可见时 | `ASSIGNED / HOST_SUPERSEDED_ADOPTED` | `MOVE_SUPERSEDE(ADOPTED)` | 视新 host |

反向事件也必须列全：未被新 host 采用的旧 Assigned lower 写 `REOPENED / HOST_SUPERSEDED_RELEASED`；规则升级重审旧墓碑先 `REOPENED / RULE_VERSION_REEVALUATION`，禁止 Tombstoned 直跳 Assigned。锚：`chanlun/review-results/unassigned-ledger-production-spec-20260713.md:264-277`。

## 9. 消解优先级与落地任务切分建议

### P0 — 先建立可度量生产基线

**任务 A：#58 + #63 同 seam 生产化。**

- 范围：`MoveViewMaterial + lower settlement + Move events + Unassigned events` 同一纯查询；塔只读。
- 验收：全 visible lower 通过 `Assigned xor Unassigned xor Tombstoned`；Pending 单列；k1..3 复现本文基线；生产路径没有把 WindowUnit 直接冒充 CompletedMove。
- 无 repaint 门：prefix equality、entry delay、原子 supersede 全过。
- 依据：#63 已要求 MoveView 与 Unassigned 共用唯一 seam：`chanlun/review-results/unassigned-ledger-production-spec-20260713.md:23-26,300-374`。

### P1 — 中枢延伸生产化

**任务 B：延伸 membership 与账本原子事件。**

- 验收：复现 `2,644 / 674`；逐对象输出 seed、absorbed range、break_at、host version；禁止越下一 seed/组。
- 反例集：核心不重叠、只外围接近、越界吞并、固定 as-of 未来追加。
- 预期收益：最大、定义锚最直接、无 direction 新裁决。

### P2 — 趋势型 CompletedMove 补全

**任务 C1：direction provider 裁决与敏感性重放。**

- 验收：唯一版本化 provider；首项占位不能充当裁决；重跑 139 个结构趋势与孤儿 membership。

**任务 C2：唯一 A/C pairing + MACD hook。**

- 验收：所有 139 个结构 Trend 逐个落入 Completed 或明确 PendingReason；不允许默认 pair；`judge_at` 与下一 bar 消费门可审计。
- 输出：247 个漏段从 Pending coverage 转为 Completed coverage 的真实数量，不报条件上界。

### P3 — 稳定约束跨窗重组

**任务 D1：选择目标重写。**

- 硬门：`regressed_to_unassigned==0`、per-level/global XOR 完整、旧 Completed consumer 不倒退。
- 优化：全域净减少优先，supersede/churn 最小化，其后才是 gap coverage。
- 验收：报告 gross resolved、released、net、Completed 四数；不得只报 4,457 seed coverage。

**任务 D2：事件计划 dry-run。**

- 对每个候选输出完整 `MOVE_SUPERSEDE + ADOPTED/RELEASED + ASSIGNED/REOPENED` 批次，不落生产账本。
- 验收：固定前缀重放 bit-exact；跨版本旧 as-of 不变；若无法得到无回退非零解，路线保持实验。

### P4 — #53 与 C2 的显式 join

**任务 E：202 个新入口的 coverage 影响重放。**

- 先把最新 P53 报告/事件谱系落回主树并核实 commit，不依赖 `/private/tmp` 作为生产事实源。
- 逐对象 join 202 个新增 entry 到同 as-of MoveView，报告 `CompletedDistinct / Pending / existing coverage gap`，禁止把 gap 计作“新增孤儿”。
- 验收：不伪造 193 个对象的背驰字段；不改变历史 `CandDeltaEvent`。

### P5 — 在线结算与墓碑

**任务 F：tail/finalization 状态机重放。**

- 尾段使用通用 as-of append 流，不增专用规则。
- 墓碑必须注入显式 `CoordinateFinalized` 测试事实；分别验证五门任一缺失都拒绝。
- 验收：开放流 tombstone 恒 0；最终化测试中每个残余恰为 Assigned 或 Tombstoned，旧版本可按 as-of 还原。

## 10. 附录：重放命令与输出摘要

### 10.1 隔离边界

```bash
# 只复制 #58 原型到 /private/tmp；数据缓存只读复用
mkdir -p /private/tmp/task64-orphan-replay/analysis
cp -R /private/tmp/c58-assembler-work/rust /private/tmp/task64-orphan-replay/
ln -s /private/tmp/c58-assembler-work/analysis/data_cache \
  /private/tmp/task64-orphan-replay/analysis/data_cache

cd /private/tmp/task64-orphan-replay/rust
CARGO_TARGET_DIR=/private/tmp/c58-assembler-work/rust/target \
  cargo build --release --features backtest_bin --bin task64_orphan_paths

/private/tmp/c58-assembler-work/rust/target/release/task64_orphan_paths \
  > /tmp/task64-orphan-paths.log 2> /tmp/task64-orphan-paths.err
```

诊断源码保留在 `/private/tmp/task64-orphan-replay/rust/src/bin/task64_orphan_paths.rs`；仓库内临时副本已删除。生产目录写入核验：

```bash
git status --short -- rust formal .chanlun/definitions
# 无输出
```

### 10.2 输出摘要

```text
DATA bars=4613599 as_of=4613598 l0_segments=40003

EXTENSION total=2644 completed=674
TREND_HOST orphan_lower=247 completed_trend=0 overlap_with_extension=79

RECOMPOSE local_seed_upper=4457 seed_covered=4457
TARGET baseline_unassigned=313 recomposed_unassigned=205
TARGET resolved_any=136 resolved_completed=29 regressed=28 net_reduction=108
GLOBAL baseline_unassigned=2028 recomposed_unassigned=2001
GLOBAL resolved_any=551 resolved_completed=182 regressed=524 net_reduction=27
PREFIX_GUARDS=9/9 pass
```

原始逐级行：`/tmp/task64-orphan-paths.log:1-7`。

### 10.3 重复性

```bash
/private/tmp/c58-assembler-work/rust/target/release/task64_orphan_paths \
  > /tmp/task64-orphan-paths-2.log 2> /tmp/task64-orphan-paths-2.err
cmp /tmp/task64-orphan-paths.log /tmp/task64-orphan-paths-2.log
shasum -a 256 /tmp/task64-orphan-paths.log /tmp/task64-orphan-paths-2.log
```

输出：

```text
a272f35ff4259f9a9db56be777a58675635011888406807cb32c701186a1919b  /tmp/task64-orphan-paths.log
a272f35ff4259f9a9db56be777a58675635011888406807cb32c701186a1919b  /tmp/task64-orphan-paths-2.log
```

两次 stderr 均为 0 行。

### 10.4 #53 发现命令

```bash
rg --files chanlun/review-results | rg 'p53|entry-relax'
# 主树无输出

rg -n -i 'CandDeltaEntryEvent|p53-entry|P53：|P53:' \
  .chanlun/goals/events.jsonl
# 无当前 2026-07-12 P53 命中
```

因此 #53 数字引用 `/private/tmp/p0-replay-work2/...` 的最新保留报告，并在正文显式标注其主树谱系缺口。

## 11. 最终回答六问

1. **中枢延伸**合法吸收跨窗界 2,581、exact-three 非种子 59、尾段 4；Assigned 下界 2,644，Completed 下界 674。应在消费层生产化，不动塔。
2. **趋势补全**已使 247 个漏段进入 139 个 Pending Trend host；Completed=0。先裁方向与唯一 A/C，再真算 MACD；当前只能报条件上界 247。
3. **跨窗界重组**可同时 seed-cover 4,457，但大规模改窗。实测 #61 目标净减少 108，全 lower 净减少 27；无新增未归属约束下的生产安全非零下界尚未建立。
4. **#53 新增 202 入口**直接既不新增归属也不新增孤儿；它只增加候选事件。必须逐对象 join C2 host 后再报连带分布。
5. **尾段 4 例**当前均为 Assigned-to-Pending，随新 lower/后继/完成事件用既有 supersede 状态机结算；无需新机制，开放流不得 tombstone。
6. **残余终态**：当前基线 U=313/T=0；实验重组 U=205/T=0。在线保持 Unassigned；只有五门全过才 Tombstoned。`ADOPT_ORPHAN` 必须与 Move create/supersede 和 C 账本 assignment 原子关联。
