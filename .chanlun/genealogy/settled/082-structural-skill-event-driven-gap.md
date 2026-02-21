---
id: "082"
title: "结构 skill 事件驱动孤岛：D策略——半事件驱动 + 诚实降级"
type: 选择
status: 已结算
date: "2026-02-21"
negation_source: heterogeneous
negation_model: gemini-3.1-pro-preview
negation_form: expansion
depends_on: ["075", "076", "078", "041", "018"]
related: ["057", "058", "069", "079", "081"]
negated_by: []
negates: []
provenance: "[新缠论]"
---

# 082 — 结构 skill 事件驱动孤岛：D策略决断

**类型**: 选择（多路径价值判断，路由 Gemini decide）
**状态**: 已结算
**日期**: 2026-02-21
**negation_source**: heterogeneous（Gemini 编排者代理，041号）
**negation_model**: gemini-3.1-pro-preview
**negation_form**: expansion（"事件驱动"概念在平台约束下的精确降级）
**前置**: 075-structural-to-skill, 076-fractal-execution-gap, 078-break-blocking-loop-strategy-d, 041-orchestrator-proxy, 018-four-way-classification
**关联**: 057-llm-not-state-machine, 058-ceremony-as-swarm-zero, 069-recursive-topology, 079-downstream-suggestions-not-blockers, 081-swarm-continuity-strategy-d

## 背景

075号谱系（已结算）声明"结构工位从 teammate 转为 skill + 事件驱动"。ceremony 不再 spawn 结构工位，dispatch-dag 定义 event_skill_map（8 个结构 skill 的触发事件映射）。

实际状态（本次确认）：
- dispatch-dag.yaml 声明了 8 个结构 skill，每个有明确的触发事件
- hooks 层有守卫脚本（genealogy-write-guard, meta-observer-guard 等），但这些只做格式检查/提醒
- **没有 event dispatcher**——当 `file_write src/**/*.py` 事件发生时，没有代码自动调用 code-verifier skill
- 结构 skill 的 agent 文件存在，但从未被事件系统自动触发
- meta-observer-guard 默认模式（hotfix 后）已从强制阻断变为自动落标放行——实质失效

**结论：075号声明了事件驱动架构，但 runtime 连接未实现。结构 skill 处于孤岛状态。**

这是 076号 fractal execution gap 的又一个实例。

## 四个选项空间

| 选项 | 描述 | 成本 | 核心问题 |
|------|------|------|----------|
| A | hooks 层增加 PostToolUse 匹配，输出提示让 Lead 触发 skill | 低 | 仍是人工触发，提示可能被忽略 |
| B | 接受当前状态为合法降级，标记为"工程债" | 零 | 与 075号产生 spec-execution gap |
| C | 回退 075号，重新 spawn 结构 teammate | 高 | 需要 /escalate，带回旧矛盾 |
| D | hooks 检测+提示 + 更新谱系诚实说明当前是半事件驱动 | 低 | 无法保证 100% 执行，依赖 Lead 认领 |

## Gemini 决断

**选项 D（混合方案：Hooks 发出事件信号，Lead 作为调度器响应，诚实降级文档）**

### 决断内容

1. **架构方向维持 075号**：不回退 teammate 模式（C），不放弃事件触发概念（B）
2. **物理载体降级**：当前平台（Claude Code hooks 不能 spawn agent），事件总线的物理载体由代码降级为 **Lead 本身**。hooks 检测文件写入/工具调用模式，输出提示文本；Lead 在上下文中看到提示，认领并执行对应 skill
3. **诚实化**：在谱系中明确记录当前实现是"半事件驱动"，消除 spec-execution gap 带来的认知污染
4. **定位对齐**：这与 076号识别的隐性规则（"下游推论靠主动认领"）是同构的——hooks 提示是信号，Lead 认领是执行

### Gemini 推理链

1. **概念优先，不绕过矛盾**：Claude Code hooks 无法直接 spawn agent 是客观物理限制。075号的"完全自动事件驱动"在当前物理层是无法兑现的空头支票（076号 spec-execution gap）。必须直面矛盾，不假装它不存在
2. **契合 078号（建议非阻塞）与 076号（主动认领）**：hooks 输出提示是"非阻塞的建议"（078号）。Lead 在上下文中看到提示并决定调用 skill，正是 076号揭示的"下游推论靠主动认领"机制
3. **维持 075号架构方向**：相比于完全退回 teammate 模式（C，成本高且带回旧矛盾）或完全放弃事件触发（B，纯靠 Lead 记忆），D 保留了"事件 -> 触发 -> skill"的逻辑链路，物理实现降级但概念链路不断
4. **诚实化系统状态**：必须在谱系中明确记录降级状态，消除认知污染

## 同质质询判定（Claude 异质质询代理执行）

**定义回溯**：
- Gemini 引用 075号（结构 skill 转为事件驱动）：正确，075号已结算，内容与引用一致
- Gemini 引用 076号（下游推论靠主动认领）：正确，076号已结算，隐性规则与 D 方案同构
- Gemini 引用 078号（下游推论 = 建议，非阻塞）：正确，078号 D 策略已结算
- 引用合法，无误引

**反例构造**：
- Gemini 提出边界 1（平台能力升级）：若 hooks 能 spawn agent，D 自动升级为完全事件驱动，决断方向不变。边界有效，不构成反例
- Gemini 提出边界 2（Lead 认知疲劳）：若 Lead 持续忽略提示，D 退化为 B。边界有效，是具体可观测条件，不是循环论证
- meta-observer-guard 历史教训（强制变自动放行）恰好证明：D 的"提示 + Lead 认领"比"强制阻断"更稳健，因为强制机制更容易因兼容性问题被 hotfix 失效

**推论检验**：
D 方案与 076/078/018 结构一致。"hooks 提示 → Lead 认领 → skill 执行"是已结算隐性规则（076号）的代码层对应，不是绕过矛盾。

**判定：决断成立。**

## 边界条件（Gemini 给出，Claude 补充）

Gemini 的边界（应触发人类 INTERRUPT）：
1. **平台能力升级**：若 Claude Code hooks 原生支持 spawn agent，立即全面升级为真正代码级事件驱动，本决断自动作废
2. **Lead 认知疲劳**：若监控发现 Lead 在实际运行中持续忽略 hooks 提示，导致结构 skill 彻底失效，必须推翻此决断，重新评估 C 方案或强制阻断

Claude 补充边界：
3. **hooks 提示频率过高**：若每次文件写入都触发提示，上下文窗口被提示文本占满，导致 token 浪费超过结构 skill 带来的价值 → 加入提示去重/冷却机制

## 风险声明

Gemini 识别的两个风险：
1. **上下文污染**：hooks 频繁输出提示文本可能占用 Lead 上下文窗口，增加 token 消耗
2. **执行不确定性**：最终调用权在 Lead 手中，存在被 LLM 概率性忽略的风险，无法保证 100% 结构约束

## 下游推论（建议，不是强制阻塞，078号）

1. **立即执行**：更新系统文档（CLAUDE.md 或 dispatch-dag.yaml 注释），将"事件驱动"描述修正为"半事件驱动（hooks 提示 + Lead 认领）"，消除 spec-execution gap
2. **立即执行**：审查现有 PostToolUse hooks，确认是否已有文件写入匹配逻辑。若无，评估添加轻量 hooks 的必要性（优先级：code-verifier 最高，因 src/*.py 写入最频繁）
3. **中期**：若 Lead 认知疲劳问题出现，引入"skill 调用日志"机制（hooks 检测 skill 是否被调用，写入轻量标记文件），提供可观测性
4. **长期**：若平台升级支持 spawn agent，直接升级为完全事件驱动，废除本决断的降级说明

## 影响声明

- 新增 082 号谱系（本文件）
- 不修改 075号已结算决断（075号的架构方向维持，只是降级了物理实现的描述）
- 更新 CLAUDE.md 或 dispatch-dag 注释以诚实描述当前实现状态（中期执行项）
- 涉及模块：dispatch-dag.yaml、hooks 层（PostToolUse）、CLAUDE.md 元编排规则说明
