# RTAS-Goal 持久化架构改造设计（D′）

**日期**: 2026-06-27
**状态**: 设计已获编排者批准（含 meta-lead 基因组改动），待写实现计划
**性质**: 蜂群基础设施改造，触动基因组（meta-lead.md），020 阻断等待已由编排者解除
**多模型对审**: Opus（Lead）提案 + Codex 两轮异质评审 + 编排者 RTAS 纠正

---

## 1. 背景与动机

### 1.1 现状三问题（活体诊断）
1. **Lead 无持久状态**：meta-lead 是"中断路由器"，每 session 从文件系统重新 ceremony 恢复，无持久身份。
2. **state 易过时**：持久化核心是 state 快照（`.chanlun/.interrupt-point.md` + `sessions/`），但快照会过时——活体证据：interrupt-point 停在 `HEAD=423597858e`，实际 git 已多个 commit 后。本次恢复差点照搬过时 session，靠"核 git 真相"纪律才避免漏一轮工作。
3. **/goal 无持久化**：`/goal` 只有命令协议（`.claude/commands/goal.md`），无状态文件，关机即丢——下 session 不知道在追什么 goal。
4. **三机制分离**：ceremony 冷/热启动、session 快照、/goal 命令是三套独立的状态/恢复机制。

### 1.2 编排者洞察
持久化核心应是 **goal**（稳定方向，直到达成不变）而非 **state**（易过时快照）。

### 1.3 编排者 RTAS 纠正（关键框架修正）
> "RTAS 蜂群架构很关键，不只是 lead。Lead 实际上只是蜂群的一个特殊的 bootstrap 功能，以及跟我对话的渠道。这就是为什么 lead 不做实际认知工作。"

含义：Lead **不是**蜂群中心或改造主体。**RTAS 蜂群（069 递归拓扑异步自指）才是认知主体**。Lead 只是 (1) bootstrap 功能（069 Swarm₀ 创世入口）+ (2) 编排者对话渠道。

---

## 2. 核心命题

> **goal 是 RTAS 蜂群的动态根**（genome=CLAUDE.md 是创世静态根，goal 是运行时动态根）。
> **蜂群的递归展开本身 = goal reducer 的确定性运行**（不是 Lead 的认知）。
> **Lead = bootstrap actuator + 编排者 IO 渠道**（不是 reducer、不是主体）。
> **持久化核心 = goal contract + events，state 降为 projection/cache**（不再是真相源，消除"状态过时"）。

这不是"给蜂群加 goal 功能"，而是揭示**蜂群本来就该是 goal reducer**——dispatch-dag.yaml 已把元编排称为"蜂群 DAG 根节点"、ceremony_scan 已是"bootloader"、已有 `topo_address`/`parent_callback` 基因。D′ 把这些显式化为 goal 驱动，并把 Lead 从隐性"准主体"诚实降为 bootstrap/IO。

---

## 3. 架构组件（isolation + 清晰边界）

| 组件 | 文件 | 职责（做什么） | 接口（怎么用） | 依赖 |
|------|------|---------------|---------------|------|
| **goal 契约** | `.chanlun/goals/current.yaml` | 当前唯一运行 goal 指针 | `goal_id` + 验收标准（可证伪 checks）+ `base_head` | — |
| **goal 事件源** | `.chanlun/goals/events.jsonl` | append-only 事件流 | `GOAL_SET` / `DECOMPOSE` / `EVIDENCE` / `CHECK_PASS` / `BLOCKED` / `SUPERSEDE` / `CLOSED` | current.yaml |
| **goal 归档** | `.chanlun/goals/archive/<id>.yaml` | 闭合 goal 目的论史 | 只读归档 | events |
| **reducer 声明** | `dispatch-dag.yaml` `goal_reducer:` 段 | 声明 ready 判据 + goal 如何展开 DAG（what） | 声明式 YAML | 033 dispatch spec |
| **reducer 实现** | `goal_reducer.py`（新建） | 确定性纯函数 `(events+git+genealogy)→ready workstations`（how） | CLI/import，纯函数可测 | reducer 声明 |
| **Lead** | `.claude/agents/meta-lead.md`（改） | bootstrap actuator + IO 渠道 + 轴线汇报 | 读 goal→调 reducer→spawn→汇报 | goal 契约 |
| **sub-goal 树** | events 子节点 | RTAS 自指递归层级 | 父子 containment 树 + blockedBy 执行 DAG | topo_address/parent_callback |

### 3.1 reducer 承载裁决（编排者授权 Lead 自决）
**声明/实现分离**（深模块原则）：
- `dispatch-dag.yaml` 的 `goal_reducer:` 段 = 可读契约（what：什么是 ready、goal 如何展开）
- `goal_reducer.py` = 可测纯函数（how：确定性 reduce 算法）
- `ceremony_scan.py` 降为 seed bootloader——它自述"硬编码优先级扫描非 DAG 拓扑"，有技术债；goal_reducer.py 取代其"评估工位"职责，顺带还债

纯函数是"确定性"的保证：同样 goal+事实 → 同样 workstations。

---

## 4. 数据流

```
genome(CLAUDE.md静态根) + goal契约(动态根) + git(地面真相,做到哪) + genealogy(发现史)
  → goal_reducer(蜂群递归展开的确定性规则，非 Lead 认知)
  → ready workstations(蜂群从 goal 递归展开，RTAS 异步自指)
  → 工位产出 → EVIDENCE/CHECK_PASS events 写回 events.jsonl
  → goal projection 更新(current.yaml)
  → session/interrupt/tasklist = projection/cache(非真相源)
```

恢复时：reducer 从 goal契约 + git + genealogy **重算** ready workstations，**不信任** state 快照（快照仅在 `base_head` 匹配时作 hint）。这是消除"状态过时"的关键——state 不再是真相源，过时也无害。

---

## 5. 三机制统一

`/ceremony`（恢复/bootstrap 入口，读 current goal，无则从 roadmap/中断点推导候选）+ `/goal`（设置/更新 current goal）→ **调用同一 reducer**。
- 069 Swarm₀ 创世 Gap 仍存在：ceremony 是 bootstrap 残余，不消失（codex 提醒）。
- 562 结构工位 bootstrap 强制保留。

---

## 6. 最小基因组改动（meta-lead.md，编排者已批准）

**必改**（基因组）：
- 开头："你是 RTAS 的 bootstrap/IO actuator，不是蜂群主体，不是 reducer。"
- 开端：从"读 session 快照直接继续"改为"读 goal contract/projection → 调确定性 reducer → spawn ready tasks → 汇报编排者"。
- "你不做的事"补："不定义 goal、不分解 goal、不评估目标价值"。
- 文件系统输入加 `.chanlun/goals/current.yaml`。
- 轴线汇报加 current goal、验收状态、blocker。
- **修现有矛盾**（codex 发现）：中断 #3 "实现分歧→你裁定或建议方案"与"不做认知工作"冲突 → 改为"路由到审查/决策工位；概念分歧上浮"。

**不碰 meta-lead 可先做**（降 020 风险）：
- 新增 `.chanlun/goals/` schema + events.jsonl
- 改 `/goal`：写 current goal + 验收标准 → 进同一 loop
- `ceremony_scan.py`：在 roadmap 之前读 current goal（roadmap 仍是 backlog/战略队列，goal 是当前交易性承诺，081 roadmap 对象化先例）
- `/ceremony`：先读 goal，无 goal 才从 roadmap/session/interrupt 推导
- session/interrupt 加 `goal_id`/`base_head`/`derived=true`，恢复时不匹配即降级为参考

---

## 7. 分层防污染（codex 关键提醒）

- **goal events** = 目的论运行史（`.chanlun/goals/`）
- **genealogy** = 概念/定义/张力生成引擎（`.chanlun/genealogy/`）
- **两者分离**：goal events 不写入 genealogy。只有 goal 的**定义/验收/分解原则发生语义变更**才升格为 genealogy 记录（用 `goal_event_id` 反向引用）。
- 依据 575（谱系 id 单写者）：避免 goal events 污染 genealogy 的单写者机制。

---

## 8. sub-goal 树（RTAS 自指）

- **containment 树**：`parent_goal_id → child_goal_id`（对应 RTAS 递归层级，topo_address）
- **execution DAG**：`blockedBy`（兄弟节点/审查/异质审计/结晶节点依赖）
- 子蜂群有 `sub_goal`（goal event 子节点，非独立 genealogy 条目）
- 子 goal 达成 → 写回 `child_goal.completed`（携 artifacts/audit refs/summary/parent_callback）→ 父 reducer 更新父 goal projection
- 子蜂群完成事件触发父蜂群 quality/genealogy/meta 处理（dispatch-dag.yaml 已有基因）

---

## 9. 错误处理

- **base_head 不匹配**：state 快照降级为参考（不信任），reducer 从 goal+git+genealogy 重算。
- **goal 事件冲突**（并发写）：events.jsonl append-only + 单写者（Lead bootstrap 时序化），冲突走 575 单写者机制。
- **reducer 失败**：回退到 ceremony_scan seed bootloader（兼容路径），报 blocker。
- **goal 无验收标准**：拒绝 GOAL_SET（验收必须可证伪，否则永动空转——lesson 0011 锋利问题②）。

---

## 10. 测试

- **reducer 纯函数单测**：构造 goal events + 模拟 git/genealogy 快照 → 断言 ready workstations（确定性：同输入同输出）。
- **恢复 e2e**：模拟"关机"（无 state 快照）→ 从 goal+git 恢复 → 断言 workstations 正确。
- **过时 state 降级测试**：base_head 不匹配 → 断言 state 被降级为参考、reducer 重算。
- **sub-goal 回写测试**：子 goal completed → 断言父 projection 更新。

---

## 11. 谱系依据

- **069**：递归拓扑异步自指蜂群——蜂群是主体（reducer=蜂群展开的依据）
- **033**：Lead=DAG/spec 解释器（不是 reducer 主体，是执行器）
- **624 / teach-0004a**：AGENT×持续监控象限无承载者——goal 持久化是其根本承载（蜂群跨 session 持续追求同一 goal）
- **562**：bootstrap 结构工位强制保留
- **162**：持久化≠进程存活（state 降 projection 的依据）
- **081**：roadmap 对象化先例（goal vs roadmap 分层）
- **575**：谱系 id 单写者（分层防污染依据）
- **本设计待结晶谱系号**：RTAS-goal 持久化架构（goal=动态根，state 降 projection，Lead=bootstrap/IO）

---

## 12. 影响声明

**新增**：`.chanlun/goals/{current.yaml,events.jsonl,archive/}`、`goal_reducer.py`、`dispatch-dag.yaml` 的 `goal_reducer:` 段
**改动**：`.claude/agents/meta-lead.md`（基因组）、`.claude/commands/{goal.md,ceremony.md}`、`scripts/ceremony_scan.py`（降 bootloader）、session/interrupt 格式
**影响模块**：整个蜂群恢复/调度机制；Lead 角色定义；持久化真相源
**不影响**：缠论领域定义/形式化（genealogy 与 goal events 分层隔离）；当前 /goal 执行中的 M1 形式化主线（架构改造是元层，与执行层并行）
