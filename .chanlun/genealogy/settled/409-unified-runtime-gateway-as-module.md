---
id: '409'
number: 409
title: 统一运行时裁决——Gateway 作为逢亮内部 channel adapter 模块
type: grammar-record
status: 已结算
date: 2026-03-10
depends_on:
  - '069'   # 递归拓扑异步自指蜂群
  - '020'   # blocking-wait 原则
  - '093'   # 五约束有向依赖图
negation_source: heterogeneous
negation_form: selection-resolution
topo_effect: "resolve:codex-gemini-divergence:unified-runtime → daemon=DAG中的liveness节点，session=一次性工作脉冲，handoff schema=唯一缺失组件"
tensions_with: []
---

# 409号：统一运行时裁决——Gateway 作为逢亮内部 channel adapter 模块

## 背景

v211-swarm 三方交叉验证（Codex + Gemini + openclaw-analyst）分析了 OpenClaw 范式与 NewChanlun RTAS 蜂群的关系。五个维度中四个收敛，一个分歧：

| 决策 | 共识 |
|------|------|
| fork OpenClaw 做 RTAS 核心 | 否定（树形 subagent 致命） |
| 独立 Python daemon | 肯定 |
| 保持 RTAS DAG | 肯定 |
| ACP 桥接 channel 层 | 肯定 |
| 逢亮的位置 | **分歧**：Codex=双进程 / Gemini=统一运行时 |

## 编排者深层推导（超越 Codex-Gemini 分歧）

三方对审后，编排者进一步推导出更深层的架构：

### Daemon = 持续唤起 CC session 的响应式 DAG 调度器

```
while True:
    context = read_handoff_schema()       # 纲目 + DAG状态 + 上一session结论
    result = invoke_cc_session(context)    # CC做认知判断
    handoff = parse_session_output(result) # DAG变更 + 任务 + tuché标记
    write_handoff_schema(handoff)          # 持久化
```

**关键洞察**：
- Session 的短暂性不是缺陷，是特性——每个 session 是一次性工作脉冲
- CC 本身就是识别涌现的机制——不需要 daemon 做二值判断
- Daemon 不持有认知权威（CC的）、不持有概念判断（agent team的）、甚至不持有状态——无状态节点
- 挂了重启，读 schema，接着跑

### Daemon 不是中心，是 liveness 节点

所有节点（CC、Gemini、Codex、daemon）都是 DAG 拓扑中的对等参与者。Daemon 的唯一特权是 **liveness**——它是那个永远醒着的节点。这个特权不是架构赋予的，是**物理属性**赋予的：它是一个进程，不依赖 session。

这解决了一个潜在矛盾：如果 daemon 是中心化的 control plane，它挂了就是全局故障，且违反 069号 RTAS 的 DAG 平等性。但如果它只是拓扑中的一个持久节点，它的失败和 Gemini API 超时没有本质区别——都是某个节点暂时不可用，恢复后从持久化状态继续。

### 唯一缺失组件：Session Handoff Schema

现有系统已覆盖大部分：
- **上下文注入**：纲目（路径定义）+ block topology DAG 状态 + events immutable 历史
- **输出协议**：consensus ceremony（共识/让步/分歧）+ stance-differential
- **Tuché 识别**：structural forcing 判据已定义

缺失的是一层薄序列化：把这些协议从 SKILL.md 自然语言描述转为机器可读的固定 schema（YAML/JSON），daemon 能 parse、CC 能读写。

### Gateway 在新架构中的消解

Gateway 只是 daemon 的一个输入源——外部消息从 WhatsApp/Telegram 进来，变成 DAG 里的事件。和 agent 返回结果没有本质区别。把其中一个事件源拆成独立进程，没有理由。

## 编排者裁决（原始）

**采纳 Gemini 3-C：逢亮统一运行时，gateway 作为内部 channel adapter 模块。**

### 三层论证

**第一层：枚举性复杂度不构成主体**

Channel 适配的"复杂度"是枚举性的，不是生成性的。WhatsApp 回复线程、Telegram inline keyboard——有限的格式差异，可以用 codec 插件穷举，不会产生自己的否定运动。一个 codec 表不构成一个主体。

**第二层：编排主体唯一性**

如果 gateway 发展出路由判断和状态管理，它就变成第二个编排者。OpenClaw 的 tree-model subagent 结构被否定，正是因为编排主体必须唯一。让 gateway 独立运行并拥有自己的决策逻辑，等于在 RTAS 的边缘偷偷复活了刚被杀死的树形模型。

**第三层：020号向心原则**

020号 blocking-wait 原则要求复杂性集中在编排层而非分散到外围。Gateway 的"智能化"是复杂性的离心运动，与 RTAS 的向心结构矛盾。

### Codex 故障隔离关切的回应

Codex 建议双进程是为了故障隔离（Node gateway crash 不影响 Python daemon）。编排者回应：用**内部模块边界**（而非进程边界）来实现隔离。如果未来经验证明确实需要拆分，那时候再拆是**顺势而为**；现在拆是**过早的工程投机**。

## 架构决策

| 组件 | 位置 | 说明 |
|------|------|------|
| RTAS 蜂群编排 | 逢亮核心 | 唯一编排主体 |
| Channel adapter | 逢亮内部模块 | codec 插件枚举 channel 格式差异 |
| ACP bridge | 逢亮内部模块 | 粗粒度任务级 RPC，接入外部 IDE/工具 |
| Ceremony | 逢亮内部 | 创世 + 热启动 |
| 拓扑计算引擎 | 逢亮核心 | 穿越步进 + K_active/S_net |

## 新原则（语法记录）

4. **枚举性复杂度不构成主体**：可以用有限 codec 穷举的格式差异不产生否定运动，不需要独立的决策层
5. **复杂性离心与 RTAS 向心矛盾**：RTAS 的结构要求复杂性集中在编排层（020号），将智能分散到外围模块违反向心原则
6. **进程边界是工程投机，模块边界是充分回应**：故障隔离不等于进程隔离；内部模块边界在当前规模下充分，未来经验驱动的拆分是顺势而为
7. **Daemon 的唯一特权是 liveness，不是 authority**：daemon 不持有认知权威、不持有概念判断、不持有状态——它是 DAG 中的对等节点，区别仅在于物理属性（永远醒着）
8. **Session 短暂性是特性，不是缺陷**：每个 CC session 是一次性工作脉冲，状态全部持久化在 DAG 和纲目里

## 边界条件

- 如果 channel adapter 的 codec 数量爆炸到无法枚举（数百种），可能需要引入动态 codec 注册——但这仍然是逢亮内部的插件机制，不是独立进程
- 如果逢亮 daemon 的 Python GIL 成为 channel 高并发瓶颈，可以用 asyncio 或 multiprocessing 在进程内解决，而非分裂为双进程
- 如果未来某个 channel 需要 Node.js 原生绑定（如 WhatsApp Web 的 Puppeteer），可以作为逢亮的子进程 worker，不是独立 daemon

## 下游推论

1. **Session Handoff Schema 定义**：将 ceremony_scan 输出 + consensus ceremony 输出 + DAG 变更指令统一为一个固定 schema
2. **Daemon 主循环实装**：while true → read schema → invoke CC → parse output → write schema
3. **CC 输入/输出协议形式化**：每次唤起时灌什么（纲目+DAG+上session结论+pending agent结果），session 结束时输出什么（DAG变更+任务+tuché标记+升级标志）
4. Channel adapter 作为事件源之一接入 daemon 事件循环
5. ACP bridge 同上——另一个事件源
6. Daemon 作为 systemd/launchd 服务部署（参考 OpenClaw daemon 适配层）

## 影响声明

- 解决了 Codex-Gemini 分歧（双进程 vs 统一运行时）
- 新增三条架构原则（语法记录）
- 确定了逢亮作为统一运行时的架构方向
- 不涉及现有代码修改（架构选型决策，实现在后续 session）

## 谱系关联

- 父记录：069号（递归拓扑异步自指蜂群）
- 相关：020号（blocking-wait 原则 → 向心原则来源）
- 相关：093号（五约束有向依赖图）
- 触发事件：v211-swarm Codex-Gemini 分歧
- OpenClaw 仓库分析：openclaw-analyst + codex-review + gemini-explore 三方交叉验证
