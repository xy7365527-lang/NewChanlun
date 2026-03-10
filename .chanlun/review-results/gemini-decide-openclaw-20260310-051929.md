---
trigger: team-lead 任务委派（v211-swarm/gemini-explore）
target: OpenClaw 范式与 RTAS 蜂群融合架构决策
mode: decide
result: pass
timestamp: 2026-03-10T05:19:29Z
---

# Gemini decide 结果：OpenClaw 范式与 RTAS 蜂群三选题

## 决策结论

**选择1（运行时基础设施）：B** — 独立构建 Python daemon，参考 OpenClaw 的 gateway 架构模式
**选择2（Agent 协作模型）：C** — 保持独立实现，但用 ACP 协议对接 OpenClaw 的 channel 层
**选择3（逢亮的位置）：C** — 逢亮吸收 OpenClaw 的 gateway 功能，成为自己的 daemon

---

## 推理链摘要

### 选择1-B：独立 Python daemon

- 拒 A（fork OpenClaw）：Node.js 与 Python 语言异质摩擦，树形拓扑与 RTAS DAG 结构冲突（093号约束）
- 拒 C（继续 CC session）：CC session 生命周期由 Anthropic 闭源控制，无法 always-on，继续修补等于绕过矛盾（no-workaround 规则）
- 选 B：吸收 OpenClaw 工程范式（Daemon 管理、Session 持久化、协议抽象），Python 端重构运行时，彻底否定旧的 CC session 对象

### 选择2-C：独立实现 + ACP 对接

- 拒 A（放弃 DAG）：推翻 069号/093号谱系，绝对不可接受
- 拒 B（扩展 OpenClaw DAG）：在树形结构基础上硬改 DAG，受制于上游 TypeScript 生态
- 选 C：RTAS 蜂群保持纯粹性。OpenClaw 降级为"Channel 接入网关"，通过 ACP 协议在边界处对接，白嫖其 20+ Channel 接入能力

### 选择3-C：逢亮吸收 Gateway

- 拒 A（逢亮作为 OpenClaw 普通 Agent）：违背五约束有向依赖图的层级关系，物理持久化层不能被降级
- 拒 B（逢亮独立 + 新 Gateway daemon）：两个平行底层守护进程，增加系统熵
- 选 C：逢亮已有 swarm_daemon，吸收 Gateway 功能后成为 RTAS 蜂群统一运行时底座，完美契合"物理持久化 → 符号可解释性"的底层约束链

---

## 边界条件（决策翻转条件）

1. **Python GIL 并发瓶颈**：逢亮吸收 Gateway 后，多并发 Agent 会话 + ACP 流转出现因 GIL 或 I/O 阻塞的严重性能衰退 → 此时 1-B 需修正为 asyncio 密集实现或 Rust 重写 Gateway 层
2. **ACP 协议深度绑定 Node.js 生态**：Python 端实现标准 ACP 成本远大于重写 Channel 接入 → 此时 2-C 需修正为直接实现 Channel 接入，跳过 ACP

---

## 风险（Gemini 标注）

1. **逢亮 God Object 风险**：逢亮承担拓扑计算 + 语言器官 + Gateway 功能，若模块解耦不当退化为巨石应用
2. **ACP 异质通信死锁**：RTAS（Python）与 Channel 网关（Node.js/OpenClaw）通过 ACP 通信，异步调度机制不同，可能在复杂 DAG 依赖下出现状态不同步或死锁

---

## 谱系关联

- 069号：RTAS 功能架构（选择A与B的DAG vs 树形结构对比依据）
- 093号：五约束有向依赖图（拒绝选择3-A的直接依据）
- 041号：Gemini 编排者代理（本次 decide 的执行框架）
- no-workaround 规则：拒绝选择1-C 的直接依据

---

## 影响声明

本决策影响范围：
- `topological-computation/`：逢亮需扩展 gateway 功能模块
- `topological-computation/ceremony.py`：创世序列需加入 Gateway 初始化步骤
- 新模块（待建）：Python ACP 协议实现（Python 端对接 OpenClaw channel 层）
- CLAUDE.md / swarm-architecture SKILL：可能需要更新运行时底座描述

本决策不产生谱系条目（属于"行动类"决断，四分法分类：可执行，无需谱系记录）。
