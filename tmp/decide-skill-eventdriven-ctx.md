# 决策上下文：结构 skill 事件驱动连接未实现

## 决策类型

**选择类**（四分法 018号）：多种合理方案，需价值判断。路由 Gemini decide 模式（041号）。

## 问题陈述

递归拓扑异步自指蜂群（069号）声称有结构 skill（genealogist, quality-guard, meta-observer, code-verifier 等），但这些 skill 是否真正被事件触发运行存疑。

**075号谱系决断**（已结算）："结构工位从 teammate 转为 skill + 事件驱动"——ceremony 不再 spawn 结构工位。

**实际状态（已确认）**：
- dispatch-dag.yaml 声明了 8 个结构 skill，每个有明确的触发事件（event_skill_map）
- hooks 层有守卫脚本（genealogy-write-guard, meta-observer-guard 等），但这些只做格式检查/提醒，不自动触发 skill
- **没有 event dispatcher**——当 `file_write src/**/*.py` 事件发生时，没有代码自动调用 code-verifier skill
- 结构 skill 的 agent 文件（.claude/agents/*.md）存在，但从未被事件系统自动触发
- meta-observer-guard 通过 Stop hook 在默认模式下自动落标放行（hotfix 已让强制变为自动放行）
- meta-observer 实际执行靠 Lead 在蜂群循环中手动读取 .claude/agents/meta-observer.md

**结论：075号声明了事件驱动架构，但 runtime 连接未实现。结构 skill 处于孤岛状态。**

这是 076号 spec-execution gap 的又一个实例。

## 相关已结算谱系

- **018**: 四分法（定理/选择/语法记录/行动）
- **041**: 编排者代理（Gemini decide 模式）
- **057**: LLM 不是状态机——"主动审查"是时间幻觉，应该是事件驱动的
- **058**: ceremony 是 Swarm₀，直接递归进入工作
- **062**: 异质碰撞协议——受控碰撞，防止同质化坍塌
- **069**: 递归拓扑异步自指蜂群（两个不可消除的 Gap）
- **073**: 蜂群能修改一切，包括自身
- **075**: 结构工位从 teammate 转为 skill + 事件驱动（已结算决断）
- **076**: 下游推论执行缺口的自相似性（fractal execution gap）
- **078**: 打破阻塞死循环——D策略（下游推论 = 建议，不是强制阻塞）
- **079**: 下游推论不是阻塞器
- **081**: 蜂群持续自动化 D 策略

## 四个选项

### 选项 A：hooks 层实现（触发提示）
在 PostToolUse hooks 中检测 file_write 模式，匹配时输出提示让 Lead 触发对应 skill。

- **实现成本**：低（利用已有 hooks 基础设施，只需增加 PostToolUse hook）
- **实际效果**：hooks 输出文本提示，Lead 看到提示后手动调用 skill
- **问题**：仍然是"人工触发"，只是触发信号来自 hook 而非 Lead 主动记忆。提示可能被忽略（同 meta-observer-guard 的 hotfix 教训）
- **平台约束**：Claude Code hooks 只能运行 shell 脚本，不能直接 spawn agent

### 选项 B：接受当前状态为合法降级
承认 075号的"事件驱动"是目标架构，当前 Claude Code 的 hooks 系统不支持自动 spawn agent。承认这是平台限制，hooks 层的格式检查+提醒已是当前最佳近似。将此标记为"工程债"而非"矛盾"。

- **实现成本**：零（只需更新谱系说明）
- **实际效果**：恢复到"Lead 在蜂群循环中手动触发 skill"的模式，不声称事件驱动
- **问题**：与 075号"事件驱动"的声称产生 spec-execution gap（076号结构）
- **优点**：诚实。不制造假象。Lead 有认知负担但不会忘记（因为 ceremony 扫描是主动的）

### 选项 C：ceremony 恢复 spawn 结构工位
回退 075号，重新在 ceremony 中 spawn genealogist/quality-guard 等结构工位作为 teammate。

- **实现成本**：中（需要走 /escalate，因为修改已结算谱系）
- **实际效果**：恢复到 075号之前的 teammate 模式，结构工位有独立 prompt 和持续存在感
- **问题**：触发了 075号谱系中描述的所有原始矛盾（孤岛、prompt 膨胀、硬编码）
- **额外问题**：需要 /escalate 流程，修改已结算谱系成本高

### 选项 D：混合方案
hooks 层做轻量检测+提示，Lead 在蜂群循环中响应提示触发 skill。不恢复 spawn，但也不假装事件驱动已实现。

- **实现成本**：低（hooks 层增加匹配逻辑，更新谱系说明诚实化）
- **实际效果**：hooks 触发提示 → Lead 看到提示 → Lead 调用 skill。"事件驱动"成为半自动（hook 检测自动，执行需 Lead）
- **与 A 的区别**：D 同时要求更新描述，诚实说明当前实现是"半事件驱动"
- **与 B 的区别**：D 仍然保留 hooks 检测层，提供辅助记忆，而 B 完全放弃

## 平台约束（Claude Code 的实际限制）

1. hooks 只能运行 shell 脚本
2. hooks 可以输出提示文本（decision: allow + reason）影响 LLM 行为
3. hooks 可以阻断（decision: block）
4. hooks **不能**直接 spawn agent 或调用 LLM
5. hooks 的提示文本会出现在 Claude 的输入流中，Claude 可以据此行动
6. meta-observer-guard 的历史表明：强制阻断容易因兼容性问题被 hotfix 为自动放行，等于失效

## 系统目标（来自 CLAUDE.md 原则）

- 原则 8：对象否定对象——事件必须来自对象，不能是超时/阈值
- 原则 10：蜂群是默认工作模式，并行是默认
- 原则 11：结晶——skill 是知识维度结晶，按需加载
- 原则 15：递归拓扑异步自指蜂群是默认架构
- 078号 D 策略：下游推论 = 建议，不是强制阻塞

## 约束摘要（需要 Gemini 在决策中考虑）

1. 修改已结算谱系（075号）需要 /escalate，成本高
2. 平台限制：Claude Code hooks 不能自动 spawn agent
3. 历史教训：强制机制被 hotfix 为自动放行（meta-observer-guard 案例）
4. 078号 D 策略已将"下游推论"定义为建议而非强制，075号的下游推论（事件驱动连接）理论上也是建议
5. 076号 fractal execution gap 已经识别了"下游推论靠主动认领"这一隐性规则

## 决策问题

**如何让结构 skill 的调用方式在当前平台约束下最接近 075号声称的"事件驱动"目标，同时不制造 spec-execution gap 假象？**

请从四个选项（A/B/C/D）中做出决断，或提出新的综合选项。
