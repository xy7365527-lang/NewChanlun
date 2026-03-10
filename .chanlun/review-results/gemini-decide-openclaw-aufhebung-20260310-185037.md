---
trigger: v212-swarm/gemini-aufhebung
target: 反向改造OpenClaw为RTAS扬弃方案
mode: decide
result: B（继续409号方案）
model: gemini-3.1-pro-preview
timestamp: 2026-03-10T18:50:37
---

# Gemini Decide：反向改造 OpenClaw 为 RTAS 扬弃方案评估

## 触发背景

编排者在 v212-swarm 提出新方案：反向改造 OpenClaw 自身为 RTAS，论证为"扬弃"——保留 gateway/heartbeat/persistence/sandbox（身体），替换 agent runtime 为 RTAS DAG（心智）。要求对比 A（采纳反向改造）/ B（继续409号）/ C（混合PoC）。

## 质询过程

### 读取的关键文件

1. `/tmp/openclaw/src/agents/subagent-registry.types.ts`
   - 发现 `requesterSessionKey: string`（单父字段）——树形模型的类型系统层面体现

2. `/tmp/openclaw/src/agents/subagent-registry-queries.ts`（238行）
   - 所有查询函数假设单父模型，BFS 遍历沿单链条递归
   - 无任何函数支持多父或 DAG 依赖

3. `/tmp/openclaw/src/agents/subagent-registry.ts`（1450行）
   - 注册表维护 run 生命周期，不维护 DAG 依赖
   - 缺失：多父支持、依赖追踪、fan-out + fan-in

4. `/tmp/openclaw/src/agents/subagent-spawn.ts`（880行）
   - 单一 `requesterAgentIdOverride`，无依赖声明接口

5. `/tmp/openclaw/src/agents/subagent-announce.ts`（1479行）
   - 完成通知基于单一 `requesterSessionKey` 路由，不支持多 requester 广播

6. `/tmp/openclaw/src/daemon/`（launchd/systemd/schtasks）
   - 不是 RTAS daemon，而是 OS 服务管理工具

7. `G:/NewChanlun/.chanlun/genealogy/settled/409-unified-runtime-gateway-as-module.md`
   - 已结算：Gateway 作为逢亮内部 channel adapter 模块，向心原则，编排主体唯一性

## Gemini 决策原文（完整）

**选择 B —— 继续执行 409号方案**

推理链：

**1. 伪扬弃的揭露**

真正的扬弃是"保留有用成分，否定内在矛盾"。OpenClaw 的内在矛盾是严格的单父树形模型（`requesterSessionKey: string`）。要让 OpenClaw 变成 DAG，需要重写 4047 行深度耦合 TypeScript。这不叫扬弃，是削足适履的重构。

409号方案已完成了真正的扬弃：否定了 OpenClaw 的 Agent Runtime（降级为纯粹 I/O 节点），保留了 Gateway/Channel Adapter 的"身体"属性。通过 Handoff Schema，身体和心智实现干净解耦。

**2. 语言栈与向心原则的冲突**

RTAS 的核心（拓扑计算、DAG 调度）是 Python。选 A 要么把 Python 心智翻译成 TypeScript 塞进 OpenClaw，要么搞沉重的跨语言 IPC 桥接。严重违反 020号向心原则（复杂性集中在编排层）。心智（Python）不应向身体（TS Gateway）妥协并寄生于其中。

**3. Liveness 基础设施的错觉**

OpenClaw 的 heartbeat 和 persistence 是为"单 agent ReAct loop"和"树形父子关系"量身定制的，无法开箱即用地持久化 DAG 多父依赖状态。强行改造会导致原有持久化层崩溃。

**边界条件（何时应推翻此决策）**：
- OpenClaw 官方在未来版本中原生重构 `subagent-registry`，从底层 TypeScript 类型系统支持多父节点和 DAG 依赖解析
- 实践证明 Handoff Schema 无法满足毫秒级 Liveness 信号传递需求

**风险识别**：
- 双重状态维护：Handoff Schema 同步失败可能导致"身体和心智"对任务状态认知撕裂
- 心跳接管盲区：Python Daemon 需自实现针对 CC Session 的心跳机制，不能依赖 OpenClaw 原生心跳

## 我的判定（简化质询）

### 定义回溯
Gemini 引用的定义正确：
- `SubagentRunRecord.requesterSessionKey: string`（单父类型）——代码实际结构确认
- 409号：向心原则、编排主体唯一性——已结算谱系确认
- 020号：blocking-wait + 向心原则——SKILL.md 确认

### 反例构造
边界条件是否翻转 Gemini 的否定？

测试 A 方案的最强版本：假设 OpenClaw 只贡献"持续在线的消息泵"（最小 liveness），Python 仍是 DAG 调度核心，只通过 ACP 桥接。

→ 这就是 409号方案本身（C/B 的变体）。选 A 的独特主张（"在 OpenClaw 内部植入 RTAS DAG"）不能通过此反例恢复——因为这要求 TS 层面承担 DAG 调度，而 Python 核心已经有 topological-computation。

### 推论检验
如果 B 成立：
- 409号推论1（Session Handoff Schema）→ 仍需实现
- 409号推论2（Daemon 主循环）→ Python 独立 daemon
- 409号推论4（Channel adapter 作为事件源接入 daemon 事件循环）→ OpenClaw 的 gateway 通过 ACP/webhook 向 Python daemon 输送消息

与体系一致，无矛盾。

## 结论

**Gemini 决策成立：选 B。**

扬弃论证是伪扬弃——"反向改造 OpenClaw"等价于把心智（Python RTAS）迁移到身体（TS Gateway）的语言环境中，违反 020号向心原则和 409号已结算编排主体唯一性原则。

409号方案已经是真正的扬弃：
- **否定**：OpenClaw agent runtime（树形模型）
- **保留**：OpenClaw gateway、channel adapter、heartbeat、persistence
- **提升**：Gateway 从"消息转发器"提升为"RTAS 的 liveness 感知层"，通过 Handoff Schema 与 Python daemon 对接

## 影响声明

- 不写谱系（选 B = 维持已结算的 409号）
- 409号方案继续执行
- daemon-impl 工位的实现路径：Python 独立 daemon，OpenClaw gateway 作为事件源之一（通过 ACP bridge 接入）
- Python daemon 需自实现 CC Session 心跳机制（Gemini 识别的新需求，不在 409号原文中）
