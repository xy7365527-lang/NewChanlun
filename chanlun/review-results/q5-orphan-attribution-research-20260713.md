# Q5 孤儿段归属裁决材料（task #61）

- 日期：2026-07-13
- 角色：只读调研员
- 裁决边界：提供 Q5 材料，**不替 P0 裁决**
- 数据：BTC 1m 全历史 `4,613,599` bars，L0 `40,003` 线段，末端 `source_index=4,613,598`；与 #60 同一输入。重放锚：`/tmp/q5-orphan-replay-20260713.log:1`、`chanlun/review-results/c-ruling-evidence-replay-20260712.md:3-6,28-37`
- 写入边界：生产 Rust、定义、原文均未改；诊断程序与日志只在 `/tmp`，本报告是 task #61 唯一仓库写入。

## 0. 裁决摘要

1. **[已验证 R01]** k=1..3 共 `5,790` 个 `i += 1` 漏段。其中 `4,457/5,790=76.977547%` 至少能进入一个包含自身的、按同层真实判据成立的替代三元组；`1,333/5,790=23.022453%` 没有任何包含自身的合法三元组。另有末端不足三元组的 `4` 段，不能混进 `i += 1` 分母。逐层原数：`/tmp/q5-orphan-replay-20260713.log:2,24,46`；生产游标：`rust/src/theta_v0/classifier/recursive_tower.rs:180-207`。
2. **[已验证 R02]** k=1..3 的首个成功窗口都从下层序号 0 开始，故“可重组的序列头部残段”为 0；`+1` 漏段全部位于成功窗口之间。逐层首尾锚：`/tmp/q5-orphan-replay-20260713.log:2,24,46`。
3. **[推断 I01]** 走势分解定理对**选定规则、坐标域已经最终化的完整走势**给出无缺口覆盖义务；它不授权在线前缀把待定段强制吸收到邻近走势。第 101/102 课说唯一分解/唯一表达，见 `docs/chanlun/text/blog/101-第101课.md:52-64`、`docs/chanlun/text/blog/102-第102课.md:14-32`；第 71 课又明确允许一笔暂时“既不能说是前面……更不能说是后一段……属于待定状态”，见 `docs/chanlun/text/blog/071-第71课.md:22-28`。
4. **[推断 I02，裁决倾向]** 三案中 C（显式 `Unassigned` 追加账本）最能同时承载“最终分解应覆盖”与“当下允许待定/后继修正”，也最适配 C2 已裁的纯函数、前缀稳定与 supersede 防线。A 的永久沉默使覆盖缺口不可审计；B 的强制吸收把未知伪装成归属。**此为材料结论，不替 P0 裁决。** C2 强制项与 Q5 未裁状态见 `chanlun/escalate/c-ruling-decision-20260713.md:9-19`。
5. **[已验证 R03]** 在 #60 的 `sequence_envelope` 敏感性适配下，最终前缀残余 `313` 段正好分为 k1 `262`、k2 `45`、k3 `6`，k4..6 为 0；其中本分类为“局部可跨窗重组” `261`、“exact-three 真不可组合” `52`。重放锚：`/tmp/q5-orphan-replay-20260713.log:3,25,47,55,57,59`；#60 原总数：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:127-137`。

## 1. 漏段成因分类学与 BTC 全历史计数

### 1.1 集合与互斥判据

**[已验证：实现口径]** 生产扫描在三元组成立时 `i += 3`，失败时 `i += 1`，循环条件是 `i+2<len`；因此 #60 已把失败漏段与末端不足三元组分列。见 `rust/src/theta_v0/classifier/recursive_tower.rs:180-207`、`chanlun/review-results/c-ruling-evidence-replay-20260712.md:111-125`。本次按相同成功窗口的稳定子 ID 重建游标路径：`/tmp/c58-assembler-work/rust/src/bin/q5_orphan_replay.rs:68-78,109-126`。

为避免类别重叠，按下列优先级逐段归类：

1. **尾部未完成段**：扫描停止后剩余的 0–2 个下层单元；它不是 `+1` 漏段，另列。若存在“最后成功窗之后的 +1 漏段但能被替代三元组包含”，也归本类；k=1..3 实际没有这种子型。判据实现：`/tmp/c58-assembler-work/rust/src/bin/q5_orphan_replay.rs:109-155`。
2. **真不可组合段**：对漏段序号 `q`，枚举所有可能包含它的连续三元组起点 `j∈[q-2,q]`，若均不成立。本标签只表示**当前 exact-three 中枢种子判据**下不可组合，不证明它不能作为中枢延伸段或 CompletedMove 构成链被坐标吸收。枚举实现：`/tmp/c58-assembler-work/rust/src/bin/q5_orphan_replay.rs:80-107,128-141`；#58 的坐标吸收与本标签是不同问题：`chanlun/review-results/assembler-spec-20260712.md:88-106`。
3. **序列头部残段**：存在合法替代三元组，且位于首个已选成功窗之前。
4. **窗口间隙段**：存在合法替代三元组，且位于两个已选成功窗之间；“能重组”只证局部存在一个合法窗口，不声称替代窗口族能形成全局无缺口 exact cover，也不声称已经找到 CompletedMove host。
5. **其他**：以上均不满足，或整层没有成功窗口。分类实现：`/tmp/c58-assembler-work/rust/src/bin/q5_orphan_replay.rs:128-155`。

底层与上层没有偷换判据：k=1 复刻 L0 `Segment.direction + [lo,hi]`，成立要求方向交替且三段共同重叠；k>=2 使用上级外缘区间的三段几何共同重叠。代码依据：`rust/src/theta_v0/classifier/center.rs:136-171,174-200`、`rust/src/theta_v0/classifier/mod.rs:98-115`、`/tmp/c58-assembler-work/rust/src/bin/q5_orphan_replay.rs:80-102,185-207`。

### 1.2 k=1..3 计数

表中类别百分比的分母是该层“失败 `+1` 漏段”；“尾端不足”不进入此分母。

| k | 下层单元 | `+1` 漏段（占下层） | 头部残段 | 窗口间隙段 | 尾部 `+1` | 真不可组合段 | 其他 | 尾端不足三元组 | 重放锚 |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | 40,003 | 3,806（9.514286%） | 0 | 3,039（79.847609%） | 0 | 767（20.152391%） | 0 | 2 | `/tmp/q5-orphan-replay-20260713.log:2` |
| 2 | 12,065 | 1,397（11.578947%） | 0 | 1,044（74.731568%） | 0 | 353（25.268432%） | 0 | 0 | `/tmp/q5-orphan-replay-20260713.log:24` |
| 3 | 3,556 | 587（16.507312%） | 0 | 374（63.713799%） | 0 | 213（36.286201%） | 0 | 2 | `/tmp/q5-orphan-replay-20260713.log:46` |
| **合计** | **55,624** | **5,790（10.409176%）** | **0** | **4,457（76.977547%）** | **0** | **1,333（23.022453%）** | **0** | **4** | 同上 |

**[已验证 R04]** 两次 release 重放输出均为 59 行且 `cmp` 相等：`/tmp/q5-orphan-replay-20260713.log:1-59`、`/tmp/q5-orphan-replay-20260713.verify.log:1-59`。程序只读取 BTC 数据和 #58 临时工作树代码；入口与输出循环见 `/tmp/c58-assembler-work/rust/src/bin/q5_orphan_replay.rs:163-258`。

**[推断边界]** `4,457` 是“若允许跨越当前已选窗边界，局部有合法三元组”的上界材料，不是方案 B 的合法性证明。多个替代三元组可能互相重叠，且 CompletedMove 还要满足延伸、同向中枢、终结与 as-of 证据；这些条件见 `chanlun/review-results/assembler-spec-20260712.md:75-154`。

## 2. 原文立场三档审计

### 2.1 直接回答：是否蕴含“无永久孤儿”

**[推断结论] 有条件蕴含最终覆盖，不蕴含在线强制归属。**

- 对一个已经选定级别/递归规则、坐标域最终化的完整走势，“任何走势都可以分解成……走势类型的连接”加上后期“递归函数能将任何走势唯一地分解”，应读为最终分解覆盖整个对象域；否则“连接/分解/唯一表达”会留下未解释空洞。原文锚：`docs/chanlun/text/blog/018-第18课.md:30-36`、`docs/chanlun/text/blog/101-第101课.md:48-64`、`docs/chanlun/text/blog/102-第102课.md:14-32`。
- 该推论不说明每个 lower element 必须成为一个三段窗口种子，也不说明在线前缀的待定段应立即属于左邻或右邻。原文明示最后可保留未完成走势，且新材料到来前可能“无从判断”：`docs/chanlun/text/blog/036-第36课.md:18-20,34-36`、`docs/chanlun/text/blog/068-第68课.md:124-130`。
- 原文最接近孤儿归属的句子反而是临时不归属：某笔可处于“中间地带”，既不属于前一特征序列，也不属于后一特征序列，状态待定，后续三笔形成后才结算。见 `docs/chanlun/text/blog/071-第71课.md:22-28`。其对象层是笔/特征序列，故把它映射到高层 Unassigned 账本属于**结构类比推断**，不是逐对象定义等同。

### 2.2 三档证据表

#60 已扫描 `docs/chanlun/text/blog/` 全部 110 个 Markdown 文件，宽松候选与去重方法见 `chanlun/review-results/c-ruling-evidence-replay-20260712.md:211-225`；本次针对“孤儿/待定/唯一分解/未完成”复扫，110 文件内没有 `Unassigned/tombstone/supersede/永久无归属/事件账本` 术语，负向扫描锚 `/tmp/q5-text-negative-20260713.log:1`。

| 档位 | 文件:行号与短引 | 对 Q5 的含义 |
|---|---|---|
| **明确支持** | `docs/chanlun/text/blog/018-第18课.md:30-36`：“任何走势……分解成……走势类型的连接” | 支持最终走势存在覆盖性分解；未给在线归属算法。 |
| **明确支持** | `docs/chanlun/text/blog/036-第36课.md:18-20`：“最后的 F' 变成……未完成的走势” | 明确允许“已完成前缀 + 未完成尾部”，反对把 Pending 折成 Completed。 |
| **明确支持** | `docs/chanlun/text/blog/036-第36课.md:34-36`：“A30-1+…+A30-m30+a，a 是未完成” | 同一表达内显式保留未完成项，并说它以后完成；最接近“无沉默缺口但允许待定”。 |
| **明确支持** | `docs/chanlun/text/blog/063-第63课.md:22-26`：“只需记住……最近一个未完成” | 明确区分已经完成对象与当前未完成对象，可映射为双账本/状态分层。 |
| **明确支持** | `docs/chanlun/text/blog/101-第101课.md:52-64`：“任何走势唯一地分解”；`docs/chanlun/text/blog/102-第102课.md:20-32`：“唯一分解……唯一地表达” | 支持规则确定后最终结果不可留沉默洞；也反对任意强塞造成多解。 |
| **明确反对** | `docs/chanlun/text/blog/018-第18课.md:38-44`：“可以不结束……不断延伸” | 反对三段/两个中枢一成立就强制封闭并归邻。 |
| **明确反对** | `docs/chanlun/text/blog/068-第68课.md:128`：“是否完成，无从判断” | 反对当下无证据时伪造归属或完成。 |
| **明确反对** | `docs/chanlun/text/blog/069-第69课.md:24-26`：“暂时看成……有可修改的地方”；“完成的图形……不可能修改” | 支持 as-of 临时态、后继修正和完成后冻结；反对历史覆盖改写。 |
| **明确反对** | `docs/chanlun/text/blog/070-第70课.md:20-40`：“暂时先……等走势走出最自然的选择” | 反对方案 B 在自然选择出现前强制固定唯一邻居。 |
| **明确反对** | `docs/chanlun/text/blog/071-第71课.md:22-28`：“既不能说是前面……更不能说是后一段……待定状态” | 直接支持临时 Unassigned，直接反对“必须马上二选一吸收”；但未授权永久墓碑。 |
| **沉默** | 全量 110 文件负向扫描仅输出文件数：`/tmp/q5-text-negative-20260713.log:1` | 原文没有 ledger schema、稳定 ID、event type、supersede 或墓碑 API；这些只能是工程承载。 |
| **沉默** | #60 已指出原文与 C2 仅认识论同构、非字段同构：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:17-18,288-296` | “永久”何时成立、规则升级后墓碑能否重开，原文未规定。 |

### 2.3 原文忠实度边界

**[推断]** 原文最相容的工程不变量是：`完整域最终应可解释覆盖` + `在线前缀允许显式待定` + `新材料只能追加修正历史视图，不能伪装为当时已知`。它支持 C 的形状，不逐字段证明 C；它反对 B 的“无证据强吸收”，也使 A 的“坐标域最终化后仍永久沉默”缺乏原文忠实度。原文锚见上表；C2 无 repaint 防线见 `chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:232-250`。

## 3. 三方案判定矩阵

以下整表是**[推断评估]**，事实约束取自各单元格锚点；评分：强=自然满足；中=可实现但需额外约束；弱=核心信息缺失或与目标冲突。

| 评估项 | A：允许永久无归属（沉默） | B：强制吸收进邻近走势 | C：显式 Unassigned 账本 |
|---|---|---|---|
| 组装器纯函数性 | **强但不完备**：确定性忽略也可纯；输出无法证明 lower ledger 全覆盖。纯函数定义锚：`chanlun/review-results/assembler-spec-20260712.md:54-73`。 | **中**：固定 tie-break 可做纯函数；若借未来决定旧归属则失败，必须加 as-of/version。 | **强**：`MoveView + UnassignedView` 都由可见前缀和事件账本纯归约，未知不折成假 host。 |
| 前缀稳定性 property test | **中/表面通过**：已输出 MoveView 可相等，但沉默孤儿的状态转移无字段可比。测试要求全字段相等见 `chanlun/review-results/assembler-spec-20260712.md:66-73`。 | **弱**：后继窗口很容易改变左吸/右吸；若不追加 supersede，会回写旧链。 | **强**：固定 `as_of=t` 归约只看到 `judge_at<=t` 的 open/assign/tombstone 事件；未来只能产生新版本。 |
| supersede 账本一致性 | **弱**：没有 orphan ID，就无法表达它何时被采用、释放或墓碑。 | **中偏弱**：每次改边界都要 supersede host，并逐段说明 ADOPTED/RELEASED；强吸收会放大改写链。 | **强**：assignment 与 host supersede 用同一 correlation 原子追加；旧行不覆盖。现有最低 supersede 字段见 `chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:234-236`。 |
| 区间套查询完整性 | **弱**：无归属段对 completed-only 查询不可见，无法区分“没有结构”与“材料缺口”。查询应只读 completed views：`chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:238-250`。 | **中**：形式覆盖完整，但无定义证据的 host 会制造伪父子范围/伪背驰段。 | **强**：区间套仍只消费 Assigned+Completed；同时返回显式 coverage gap，做到“可审计完整”而不把未知当证据。 |
| #54 类计数可重放性 | **弱**：分母与失败桶会随组装规则改变而静默漂移。#60 已证明 `213/93/38/0` 不可原搬：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:67-88`。 | **中**：给定规则版本可重放，但计数混入强制归属假设；规则变更需要大规模 supersede。 | **强**：每个 lower element 在指定 as-of 都是 Assigned/Unassigned/Tombstone 之一，分母、缺口和转正数均可归约。 |
| 原文忠实度 | **弱（对“永久+沉默”）**：与最终唯一分解的覆盖义务有张力。 | **弱**：与“待定中间地带”“无从判断”“暂时划分”直接冲突。 | **强但非逐字段原文**：忠实承载 Pending→后继结算；墓碑仍需严格工程边界。原文锚：`docs/chanlun/text/blog/071-第71课.md:22-28`、`docs/chanlun/text/blog/069-第69课.md:24-26`；完整路径见 §2。 |

**[推断结论] 倾向 C，不替 P0 裁决。** C 不是因为“账本更多”而胜出，而是它唯一同时保留三值语义：`已归属 / 暂未归属 / 经最终证据不可归属`。A 把后两者压成无记录，B 把前两者压成伪归属。该三分是对 C2 `Pending/Completed + supersede` 的最小补全；现有原型已经坚持“不知道不折为 false”，见 `chanlun/review-results/assembler-spec-20260712.md:150-164`。

## 4. 若采 C：Unassigned 账本最小 schema

以下是**[草案/推断]**，不是当前生产类型。约束来源是稳定 `ElementId`（`rust/src/theta_v0/classifier/recursive_tower.rs:52-72`）、as-of 可见域（`chanlun/review-results/assembler-spec-20260712.md:54-73`）和追加式 supersede（`chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:232-236`）。

### 4.1 追加事件行

```text
UnassignedEvent {
  event_id: EventId,                    // 追加行唯一 ID
  orphan_id: (target_level, lower_id),  // 稳定归属对象；lower_id = ElementId
  generation: u32,                     // host supersede 后释放时 +1
  lower_range: [start_index, end_index],
  event_type: OPENED | ASSIGNED | TOMBSTONED | REOPENED,
  reason: ReasonCode,
  judge_at: source_index,               // 此转移最早可知时点
  host: Option<(move_id, move_version, host_completion)>,
  previous_event_id: Option<EventId>,
  correlation_id: Option<EventId>,      // 与 Move SUPERSEDE 原子关联
  evidence: EvidenceRef,                // 前后窗口/闭合事件/水位/规则版本
  rule_version: RuleVersion
}
```

最小 `ReasonCode`：`CURSOR_GAP_RECOMPOSABLE`、`NO_CONTAINING_SEED_WINDOW`、`LEADING_RESIDUAL`、`TERMINAL_INSUFFICIENT`、`NO_HOST_AT_AS_OF`、`HOST_SUPERSEDED_RELEASED`、`FINALIZED_NO_LEGAL_HOST`。前五项来自 §1 分类与 #60 末端边界，见 `/tmp/q5-orphan-replay-20260713.log:2,24,46`、`chanlun/review-results/c-ruling-evidence-replay-20260712.md:125-137`。

### 4.2 生命周期与“转正”事件

```text
UNASSIGNED --ORPHAN_ASSIGNED--> ASSIGNED(host_version)
UNASSIGNED --ORPHAN_TOMBSTONED--> TOMBSTONE(rule_version)

ASSIGNED(old_host) --MOVE_SUPERSEDE + ADOPTED--> ASSIGNED(new_host)
ASSIGNED(old_host) --MOVE_SUPERSEDE + RELEASED--> UNASSIGNED(generation+1)
```

- **[草案] 转正事件类型：`ORPHAN_ASSIGNED`。** `host` 必填，记录 host 当时是 Pending 还是 Completed；“assigned”只表示已有归属，不冒充“走势已完成”。如果转正改变既有 host 的 chain，同一 `correlation_id` 必须同时追加 `MOVE_SUPERSEDE {old_id,new_id,reason=ADOPT_ORPHAN,judge_at}`；若创建全新 host，则由新 Move 事件反向列出 adopted orphan IDs。旧版本不可覆盖。当前 supersede 最小契约见 `chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:232-236`。
- **[草案] `REOPENED` 是一致性所需的窄扩展。** 用户要求的主路径仍是 `unassigned→assigned/tombstone`；但若 host 被 supersede 且新 host 没有采用该段，不能让旧 assignment 假装仍有效，也不能删除旧行，只能开启新 generation。该要求由“历史 as-of 还原旧版本、当前查询追随 supersede 链”推出：`chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:234-236`。
- **[草案] reduce 不变量：** 对任意 `(orphan_id,generation,as_of)`，恰有一个当前状态；每个 lower element 在 coverage 查询中恰落入 `Assigned / Unassigned / Tombstone` 一类。区间套只把 `Assigned ∧ host=Completed` 当结构证据，另外返回 Unassigned/Tombstone coverage 注记。Completed-only 查询口径见 `chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:238-250`。

### 4.3 永久墓碑条件

**[草案，合取门]** 只有下列全部满足才允许 `ORPHAN_TOMBSTONED`：

1. `lower_id` 自身已经 settled，坐标不会再因 parser/上级 supersede 改写；
2. 已出现坐标在其后的 CompletedMove/封闭事件，证明候选归属区间已经被后继走势封闭；
3. 数据分区/回放域存在显式 `coordinate_finalized_at` 水位，且该水位越过所有可能包含该段的种子窗与 host 边界；普通流式“现在到末端”不等于最终化；
4. 在当前 `rule_version` 下，穷举候选后仍无合法 host，且没有 PendingMove 跨越该段；
5. `judge_at = max(lower_settled_at, successor_closed_at, coordinate_finalized_at, exhaustive_check_at)`，墓碑只从该时点以后可见。

第 2+3 条直接落实 #60 边界：“被后继走势封闭且坐标域最终化”后才可谈永久；当前原型没有墓碑类型，见 `chanlun/review-results/c-ruling-evidence-replay-20260712.md:135-137`。第 4 条防止把本次 `true noncomposable` 误读成 CompletedMove 永久不可归属：#58 可以不以其为 seed，而按坐标纳入 chain，见 `chanlun/review-results/assembler-spec-20260712.md:88-106`。**[推断]** 墓碑只对同一 `rule_version` 永久；规则迁移时若要重审，应追加新 generation/迁移事件，不得改旧墓碑。

## 5. #60 最终无归属残余：10 例画像

### 5.1 取样口径

**[已验证]** #60 因方向尚未裁定给了三种敏感性适配。本表选 `sequence_envelope`，因为其“最终无归属”总数为 313，且可由本重放逐层复核为 `262+45+6`；这只是固定样本宇宙，不表示 P0 已裁该 direction provider。#60 三适配边界见 `chanlun/review-results/c-ruling-evidence-replay-20260712.md:11-18,32-37,127-137`；复核见 `/tmp/q5-orphan-replay-20260713.log:3,25,47,55,57,59`。

“跨窗能归属”列的“是”只表示存在一个包含该段的合法替代三元组；“否”只表示 exact-three seed 不可组合，二者都不直接裁定 CompletedMove host。当前区间套只读 completed move views，见 `chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:238-250`；因此这些无 host 段当前均不可见，采 C 后应对 coverage 审计可见、但在转正且 host Completed 前仍不得充当区间套背驰证据。

| # | 位置（目标 k / lower ID / lower_idx / source range） | 成因 | 跨窗局部重组 | 当前区间套可见？ | 重放锚 |
|---:|---|---|---|---|---|
| 1 | k1 / `L0#63` / 63 / `[9285,9400]` | 窗口间隙 | 是 | 否；C 下仅 coverage 可见 | `/tmp/q5-orphan-replay-20260713.log:4` |
| 2 | k1 / `L0#1126` / 1126 / `[137974,137996]` | exact-three 真不可组合 | 否 | 否；C 下仅 coverage 可见 | `/tmp/q5-orphan-replay-20260713.log:13` |
| 3 | k1 / `L0#130` / 130 / `[18850,18911]` | 窗口间隙 | 是 | 否；C 下仅 coverage 可见 | `/tmp/q5-orphan-replay-20260713.log:7` |
| 4 | k1 / `L0#521` / 521 / `[67110,67179]` | 窗口间隙 | 是 | 否；C 下仅 coverage 可见 | `/tmp/q5-orphan-replay-20260713.log:12` |
| 5 | k2 / `L1#732` / 732 / `[284333,284668]` | 窗口间隙 | 是 | 否；C 下仅 coverage 可见 | `/tmp/q5-orphan-replay-20260713.log:26` |
| 6 | k2 / `L1#3278` / 3278 / `[1183803,1184381]` | exact-three 真不可组合 | 否 | 否；C 下仅 coverage 可见 | `/tmp/q5-orphan-replay-20260713.log:38` |
| 7 | k2 / `L1#3700` / 3700 / `[1333681,1334083]` | exact-three 真不可组合 | 否 | 否；C 下仅 coverage 可见 | `/tmp/q5-orphan-replay-20260713.log:40` |
| 8 | k3 / `L2#107` / 107 / `[145672,147050]` | exact-three 真不可组合 | 否 | 否；C 下仅 coverage 可见 | `/tmp/q5-orphan-replay-20260713.log:48` |
| 9 | k3 / `L2#1197` / 1197 / `[1453565,1454238]` | 窗口间隙 | 是 | 否；C 下仅 coverage 可见 | `/tmp/q5-orphan-replay-20260713.log:49` |
| 10 | k3 / `L2#3455` / 3455 / `[4476188,4477506]` | 窗口间隙 | 是 | 否；C 下仅 coverage 可见 | `/tmp/q5-orphan-replay-20260713.log:53` |

**[已验证]** 样本均来自 `membership(view, lower[idx]) == false` 的 residual 集合，不是仅凭坐标肉眼挑选；筛选实现见 `/tmp/c58-assembler-work/rust/src/bin/q5_orphan_replay.rs:157-161,224-257`。

## 6. 给 P0 的最小裁决句式

若 P0 采纳本材料倾向，建议裁决只写到以下强度：

> Q5 采 C：任何截至 as-of 未被 CompletedMove/PendingMove 明确归属的稳定 lower element，必须进入追加式 Unassigned 账本；禁止沉默丢失，禁止无证据强制吸收。后继证据可通过 `ORPHAN_ASSIGNED` 转正，并与 Move supersede 原子关联。仅当后继走势已封闭候选区间、坐标域最终化且当前规则下穷举无合法 host，才可追加墓碑。墓碑与归属均不得回写历史 as-of。

这是**[建议，不是裁决]**。它保持 C2 已裁的三道无 repaint 防线：`chanlun/escalate/c-ruling-decision-20260713.md:11-19`，并保留 #60 对“最终无归属≠理论永久”的边界：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:127-137,335-341`。

## 7. 可复现命令与证据锚

```bash
cd /tmp/c58-assembler-work/rust
cargo run --release --features backtest_bin --bin q5_orphan_replay \
  > /tmp/q5-orphan-replay-20260713.log \
  2> /tmp/q5-orphan-replay-20260713.err
cargo run --release --features backtest_bin --bin q5_orphan_replay \
  > /tmp/q5-orphan-replay-20260713.verify.log \
  2> /tmp/q5-orphan-replay-20260713.verify.err
cmp /tmp/q5-orphan-replay-20260713.log \
    /tmp/q5-orphan-replay-20260713.verify.log
```

- 诊断源：`/tmp/c58-assembler-work/rust/src/bin/q5_orphan_replay.rs:1-258`
- 主输出：`/tmp/q5-orphan-replay-20260713.log:1-59`
- 独立复跑：`/tmp/q5-orphan-replay-20260713.verify.log:1-59`
- #60 基线输出：`/tmp/c60-window-replay.log:1-26`
- `+1` 总数交叉核对：本报告 k1..3 的 `3,806/1,397/587` 与 #60 `chanlun/review-results/c-ruling-evidence-replay-20260712.md:111-125` 完全一致。
