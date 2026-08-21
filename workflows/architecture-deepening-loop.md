# 架构深化循环（architecture-deepening loop）

> v1，已入仓（#1159，2026-08-21）。本循环的每轮实体形态 = wayfinder 图（issue tracker 上 `wayfinder:map` 一张）+ 其 handoff（SPEC → impl 票 → sandcastle 拾取）。
> 纪律正本引用：`docs/agents/wayfinder-workflow.md`（本仓 wayfinder 唯一正本，含开场对账 / 一票一会话 / 图正文写入协议 / 并行纪律）、`docs/agents/delivery-discipline.md`、`docs/agents/issue-tracker.md`。

## 目的

把「扫描 → 候选图 → SPEC → sandcastle 实装 → 再扫描」固化成一条常驻循环，持续把浅模块加深到「扫描 top recommendation 全 Speculative 且判定不值」为止。终点是**裁决**，不是机器状态。

## 触发（事件型，不设定时器）

下一轮在以下条件**全部**满足时启动（任一不满足 → 停止，报「循环未就绪」，不动手）：

1. 上一轮 handoff 的 impl 票全部 CLOSED 且已合入 main；
2. 无 open `wayfinder:map`（不叠图）；
3. 上一轮图已关、决策索引已写全。

理由：定时空跑会扫到半改造状态或在飞图领土，图建在流沙上。

## 每轮步骤

1. **开场对账**（读，不写）：main 顶点 · open maps · open sandcastle 票 · 上一轮图的 Decisions-so-far。
2. **前置检查**：触发三条件核对，不满足即止。
3. **扫描**：跑 `/improve-codebase-architecture`——热点 = `git log` 最近改动区；先读 `CONTEXT.md` 与相关 ADR，不重诉已有裁定；产出 HTML 报告到 `$TMPDIR`（不进仓）。
4. **HITL 检查点①（候选挑选）**：用户从候选卡片挑一个 → 开新图；若 top recommendation 全 Speculative 且用户判不值 → **判停，循环结束**。
5. **图内收敛**：新图 Notes 声明「本图带实装」；决策票 grilling 收敛 → SPEC → impl 票（`sandcastle` label 入拾取队列）。
6. **sandcastle 实装**：按 #1000 口径派发，双工蜂（实装+独立评审），评审 PASS + 用户批准逐票合入。
7. **图关**：决策索引写全，handoff 落地 commit 记入图。
8. **回到触发**：等条件 1–3 重新满足。

## 检查点（push-right：一次一问，给推荐答案，用户拍板）

- **①候选挑选**（每轮扫描后）：报告 + Top recommendation，用户选一个或判停。
- **②图收口交棒**（图内）：图正文写入协议（docs/agents/wayfinder-workflow.md）走完再交 SPEC。
- **③轮终裁决**（每图关后）：再跑一轮 / 停。判停理由与依据写进最后一张图的关图评论。

## 判停条件（可证伪，不靠感觉）

- top recommendation 全 Speculative；**或**
- 用户主动停（记录理由）。

## 每轮 Brief（给用户，决策就绪摘要）

一轮结束只给：本轮图名+链接 · 决策数 · 落地 commit · 下一轮是否就绪（触发三条件状态）。不给原始扫描输出。

## 边界

- **不叠图**：在飞图存在时禁止开新图，等 handoff 落地。
- **一票一会话**：一张决策票一个会话解决，多票并行按 wayfinder 并行纪律。
- **新目的另开图**：本循环只服务「架构深化」这一个目的地；「插件化」「数据层」等新目的各自开图，不塞进本循环。
- **入仓纪律**：本循环产物（图/SPEC/票）全在 tracker；本地只落本 spec 文件与图内引用的 review-results（工作草稿，活期绑定票）。

## 实证基线（2026-08-21 首轮事实）

- 上一轮循环实例：map #1055（2026-08-18，五候选）→ SPEC #1077 → impl #1081/#1082（sandcastle 在飞）。
- 前身图 #743 端到端模块化已闭合（SPEC #756 已簿记关票 2026-08-21）。
