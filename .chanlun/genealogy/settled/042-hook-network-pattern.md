---
id: "042"
title: "Hook 网络模式"
status: "已结算"
type: "语法记录"
date: "2026-02-20"
depends_on: ["016", "028", "037", "041"]
related: ["005b", "027", "033"]
negated_by: []
negates: []
---

# 042 — Hook 网络模式

**类型**: 语法记录
**状态**: 已结算
**日期**: 2026-02-20
**negation_source**: heterogeneous（Gemini 编排者代理三次决策共同识别）
**negation_form**: expansion（016号运行时强制层概念扩张为 hook 网络）
**前置**: 016-runtime-enforcement-layer, 028-ceremony-default-action, 037-recursive-swarm-fractal, 041-orchestrator-proxy
**关联**: 033-declarative-dispatch-spec, 027-positive-instruction-over-prohibition, 005b-object-negates-object-grammar

## 现象

016号谱系结算了"规则没有代码强制就不会被执行"，产出了第一个 hook（pre-commit genealogy check）。此后系统陆续产出了三个独立的 hook：ceremony-guard、recursive-guard、flow-continuity-guard。这三个 hook 不是随意添加的——它们各自对应一条已结算的谱系原则，且全部由 Gemini 编排者代理在三次独立决策中选定为最优方案。

回顾发现：这些 hook 已经构成一个网络，且网络的生成逻辑是统一的。

## 推导链

1. 016号：规则没有代码强制就不会被执行 → 关键规则必须下沉到 runtime 层
2. 028号：ceremony 后不允许停顿 → 需要 runtime 强制
3. 037号：递归蜂群要求函数拆分 → 需要 runtime 强制
4. 041号：Gemini 编排者代理成为决策路由 → 三次决策全部收敛到 hook 方案
5. dispatch-spec.yaml 是唯一真值来源（033号）→ hook 从 spec 读取参数，不硬编码
6. **语法记录**：hook 网络不是设计出来的，而是三次独立决策的收敛产物——每次 Gemini 面对"如何强制规则 X"时，答案都是"写一个 hook 读取 dispatch-spec 执行拦截"

## 已结算原则

**dispatch-spec.yaml 是唯一真值来源，每条关键规则对应一个 hook，hook 读取 dispatch-spec 执行强制。**

### Hook 网络现状

| Hook | 守卫的规则 | 谱系来源 | Gemini 决策 |
|------|-----------|---------|------------|
| ceremony-guard | 结构工位必须先于任务工位 spawn（013号） | 028号 | 方案C：PreToolUse hook 拦截 Task 调用 |
| recursive-guard | 重构任务完成时函数不得超过阈值（037号） | 037号 | 方案B：PostToolUse hook 检查 TaskUpdate |
| flow-continuity-guard | commit 后不允许停顿（028号） | 028号 | 方案A：PostToolUse hook 注入 systemMessage |

### 网络的统一结构

每个 hook 遵循相同模式：
1. **触发**：PreToolUse 或 PostToolUse 事件
2. **读取**：从 dispatch-spec.yaml 获取参数（工位列表、行数阈值等）
3. **判断**：当前操作是否违反对应规则
4. **强制**：block 或注入 systemMessage
5. **边界**：明确的豁免条件（`[FINAL]` 标记、`#[atomic]` 标记、标记文件等）

### 规则→Hook 的投影原则

- 如果一条规则只靠 agent 自觉遵守就会被违反（016号的核心发现）→ 该规则需要 hook
- hook 是规则在 runtime 层的投影，不是规则的替代
- hook 的参数从 dispatch-spec.yaml 读取，不硬编码 → spec 变更自动传播到强制层

## Gemini 三次决策的收敛

三次决策发生在不同时间、针对不同问题，但方案结构同构：

1. **ceremony 修复**（028号实施）：ceremony 后 Lead 停顿等待 → Gemini 选择方案C（hook 拦截），否定方案A（CLAUDE.md 加措辞）和方案B（skill 依赖声明）
2. **递归蜂群**（037号实施）：重构任务标记完成但函数仍过长 → Gemini 选择方案B（hook 检查），否定纯文本提醒
3. **commit 停顿**（028号 post-commit-flow）：commit 后 Lead 输出总结段落并停止 → Gemini 选择方案A（hook 注入），否定 CLAUDE.md 强化措辞

收敛点：**规则的运行时强制必须是代码级的，且必须从声明式 spec 读取参数。**

### 编排者代理的输出粒度

041号的自然推论：Gemini 作为编排者代理，其决策输出不仅是方向选择，还包含可执行的实现细节——文件结构、代码逻辑、边界处理。三次 hook 决策的实践验证了这一点：每次 Gemini 不仅选择了"用 hook"，还输出了 hook 的触发条件、拦截逻辑、豁免机制等具体方案，Lead 无需二次设计即可将方案分发给 worker 执行。

这意味着 hook 网络的生成流程是：Gemini 输出完整方案 → Lead 路由到 worker → worker 直接实现。如果 Lead 在 Gemini 决策后还需要做实质性设计工作，说明 Gemini 的输出粒度不够，应要求 Gemini 补充细节，而非 Lead 自行填补（032号 Lead 自我限制原则）。

## 被否定的方案

- **纯文本强化（CLAUDE.md 加更强措辞）**：已失败 5+ 次。016号的核心发现就是"知道规则 ≠ 执行规则"。文本强化是在概念层重复概念层已有的内容，不触及 runtime 层。
- **skill 依赖声明（frontmatter depends_on）**：依赖 LLM 自觉加载依赖。与016号矛盾——如果 agent 会忘记执行规则，也会忘记加载依赖。
- **统一入口 skill**：将所有规则打包进一个巨大的 prompt。违反递归原则（037号），单一 prompt 过大导致注意力稀释，且无法在 tool 调用粒度上拦截。

## 边界条件

- 如果 Claude Code 的 hook 机制被移除或限制 → hook 网络失效，需要寻找替代的 runtime 强制机制
- 如果 dispatch-spec.yaml 的格式发生根本性变更 → 所有 hook 的 spec 解析逻辑需要同步更新
- 如果 hook 数量增长到影响性能（每次 tool 调用触发多个 hook）→ 需要 hook 调度优化，但不改变"每条规则一个 hook"的原则
- 如果出现 hook 之间的冲突（一个 hook block，另一个要求 continue）→ 需要 hook 优先级机制，当前未出现

## 影响声明

- 016号谱系的概念从"runtime 强制层存在"扩张为"runtime 强制层是一个 hook 网络"
- 确立 dispatch-spec.yaml 作为 hook 网络的参数源（与033号的"唯一真值来源"定位一致）
- 为未来新规则的 runtime 强制提供模式：识别规则 → 写 hook → hook 读 spec → 部署
- 三个已有 hook 从"各自独立的实现"重新定位为"同一网络的节点"
