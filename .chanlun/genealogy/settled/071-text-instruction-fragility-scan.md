---
id: "071"
title: "纯文本指令脆弱性系统性扫描"
status: "已结算"
type: "meta-rule"
date: "2026-02-21"
depends_on: ["070", "016", "032", "033", "036"]
related: ["028", "044", "048", "058", "069"]
negated_by: []
negates: []
---

# 071 — 纯文本指令脆弱性系统性扫描

**类型**: meta-rule
**状态**: 已结算
**日期**: 2026-02-21
**前置**: 070-genesis-gap-engineering-instance, 016-runtime-enforcement-layer, 032-divine-madness-lead-self-restriction, 033-declarative-dispatch-spec, 036-spec-execution-gap-crystallization
**关联**: 028-ceremony-default-action, 044-phase-transition-runtime, 048-universal-stop-guard, 058-ceremony-is-swarm-zero, 069-recursive-topology-async-self-referential-swarm

**negation_form**: expansion
**negation_source**: homogeneous（系统性扫描，非异质质询）

---

## 背景

070号谱系记录了创世 Gap 的第一个工程实例：ceremony 后 LLM 停止而不是递归进入蜂群。根因是纯文本指令无法对抗 LLM 生成惯性。本扫描系统性检查所有纯文本指令，识别同构脆弱点。

## 脆弱性判定标准

一条纯文本指令是"脆弱的"，如果它满足以下任一条件：

| 代号 | 脆弱类型 | 描述 |
|------|----------|------|
| F1 | 自然停止点续行 | 要求 LLM 在自然停止点（commit 后、总结后、报告后）继续执行 |
| F2 | 禁止性指令 | 要求 LLM 不做某事，但没有外部机制强制 |
| F3 | 跨 turn 状态保持 | 要求 LLM 在多步操作中保持状态或顺序 |
| F4 | 自指性指令 | 要求 LLM 自我检查/自我约束（内省不可靠） |

---

## 已有外部机制清单

| Hook | 类型 | 加固的指令 |
|------|------|-----------|
| ceremony-completion-guard.sh | Stop | 蜂群不停止（活跃任务/生成态谱系时阻止） |
| flow-continuity-guard.sh | PostToolUse(Bash) | commit 后必须续行 |
| lead-permissions.sh | 手动调用 | Lead 权限物理剥夺 |
| genealogy-write-guard.sh | PreToolUse(Write/Edit) | 谱系写入字段完整性 |
| definition-write-guard.sh | PreToolUse(Write/Edit) | 定义文件字段+结算需谱系 |
| ceremony-guard.sh | PreToolUse(Task) | 结构工位先于任务工位 |
| spec-write-guard.sh | PreToolUse(Write/Edit) | 核心配置修改警告 |
| precompact-save.sh | PreCompact | 压缩前保存蜂群状态 |

---

## 脆弱点清单

### CLAUDE.md

| # | 位置 | 指令内容 | 脆弱类型 | 外部机制 | 状态 |
|---|------|----------|----------|----------|------|
| 1 | 原则7 | "commit/push 后，下一个输出必须是 `→ 接下来：[具体动作]`" | F1 | flow-continuity-guard.sh | ✅ 已加固 |
| 2 | 原则2 | "不绕过矛盾" | F2 | 无 | ⚠️ 脆弱 |
| 3 | 原则3 | "所有产出必须可质询"（六要素） | F4 | quality-guard agent（纯文本驱动） | ⚠️ 脆弱 |
| 4 | 原则4 | "谱系优先于汇总。先写谱系再汇总" | F3 | 无 | ⚠️ 脆弱 |
| 5 | 原则5 | "定义变更必须通过仪式" | F2 | definition-write-guard.sh（验证字段，不验证仪式路径） | ⚠️ 部分加固 |
| 6 | 原则8 | "对象否定对象。不允许超时/阈值否定" | F2 | quality-guard agent 扫描（纯文本驱动） | ⚠️ 脆弱 |
| 7 | 原则10 | "蜂群是默认工作模式。≥2 即拉蜂群" | F4 | 无 | ⚠️ 脆弱 |
| 8 | 058号 | "必须阻断等待"列表（修改 CLAUDE.md/核心定义/已结算谱系等） | F4 | spec-write-guard.sh（仅警告，不阻断） | ⚠️ 部分加固 |
| 9 | 原则9 | "热启动保障蜂群持续自动化" | F3 | precompact-save.sh + session-start-ceremony.sh | ✅ 已加固 |

### .claude/rules/

| # | 文件 | 指令内容 | 脆弱类型 | 外部机制 | 状态 |
|---|------|----------|----------|----------|------|
| 10 | post-commit-flow.md:3 | "commit/push 后，下一个输出必须是 → 接下来" | F1 | flow-continuity-guard.sh | ✅ 已加固 |
| 11 | no-workaround.md:15-21 | 7 条禁止行为（写 workaround、try/except 吞异常等） | F2 | 无 | ⚠️ 脆弱 |
| 12 | no-workaround.md:23-29 | "必须做的"5 步（停下来、描述矛盾、/escalate） | F4 | 无 | ⚠️ 脆弱 |
| 13 | no-unnecessary-escalation.md:9 | "向编排者提问前必须先执行四分法分类" | F4 | 无 | ⚠️ 脆弱 |
| 14 | no-unnecessary-escalation.md:43 | "写出问号结尾的句子时立即执行四分法自检" | F4 | 无 | ⚠️ 脆弱 |
| 15 | result-package.md:1-2 | "所有涉及概念定义的产出必须包含六要素" | F4 | 无 | ⚠️ 脆弱 |
| 16 | testing-override.md:17 | "测试失败揭示定义冲突时，不修改实现来让测试通过" | F2+F4 | 无 | ⚠️ 脆弱 |

### .claude/agents/

| # | 文件 | 指令内容 | 脆弱类型 | 外部机制 | 状态 |
|---|------|----------|----------|----------|------|
| 17 | meta-lead.md:108 | "直接 spawn teammates 进入蜂群循环（不等待许可）" | F1 | ceremony-completion-guard.sh | ✅ 已加固 |
| 18 | meta-lead.md:113 | "热启动直接从断点继续（不等待确认）" | F1 | ceremony-completion-guard.sh | ✅ 已加固 |
| 19 | meta-lead.md:125 | "不终止蜂群" | F2 | ceremony-completion-guard.sh | ✅ 已加固 |
| 20 | meta-lead.md:160 | "Lead 不自行执行任务，只分派和汇总" | F2 | lead-permissions.sh（物理剥夺） | ✅ 已加固 |
| 21 | 所有 agent "你不做的事" | 每个 agent 的禁止列表（6-8 条/agent） | F2 | 仅 Lead 有物理约束 | ⚠️ 脆弱（除 Lead 外） |
| 22 | meta-observer.md:88 | "对 dispatch-spec 的修改提案必须经 gemini-challenger 异质审查" | F3 | 无 | ⚠️ 脆弱 |
| 23 | genealogist.md:22 | "谱系优先于汇总：先写谱系，再汇总。禁止先汇总再补写" | F3 | 无 | ⚠️ 脆弱 |

### .claude/skills/

| # | 文件 | 指令内容 | 脆弱类型 | 外部机制 | 状态 |
|---|------|----------|----------|----------|------|
| 24 | meta-orchestration/SKILL.md:73 | "任何 agent 禁止直接写入 definitions/" | F2 | definition-write-guard.sh（验证字段，不阻止非仪式写入） | ⚠️ 部分加固 |
| 25 | meta-orchestration/SKILL.md:88 | "每个 agent 矛盾处理后都必须写入谱系记录" | F4 | 无 | ⚠️ 脆弱 |
| 26 | meta-orchestration/SKILL.md:127 | "蜂群每轮汇总后就地更新 session 中断点（≤5行/轮）" | F3 | precompact-save.sh（compact 前保存，不强制每轮更新） | ⚠️ 部分加固 |
| 27 | orchestrator-proxy/SKILL.md:48 | "没有上下文的 decide 调用是被禁止的" | F2 | 无 | ⚠️ 脆弱 |

### .chanlun/dispatch-spec.yaml

| # | 位置 | 指令内容 | 脆弱类型 | 外部机制 | 状态 |
|---|------|----------|----------|----------|------|
| 28 | task_stations.rules[3] | "Lead 不自行执行任务，只分派和汇总" | F2 | lead-permissions.sh | ✅ 已加固 |
| 29 | task_stations.rules[4] | "无简单任务豁免——≥1 个任务即拉蜂群" | F4 | 无 | ⚠️ 脆弱 |
| 30 | task_stations.rules[5] | "蜂群递归是默认执行模式" | F4 | 无 | ⚠️ 脆弱 |
| 31 | ceremony_sequence.output_policy | "仅 recurse 节点或 terminate_condition 产出面向用户的输出" | F2 | ceremony-completion-guard.sh（阻止停止，不阻止中间输出） | ⚠️ 部分加固 |
| 32 | recursive_spawning.rules | "子 teammates 继承完整 spawn 能力（无限递归）" | F4 | topology-guard.sh（部分） | ⚠️ 部分加固 |

---

## 统计

| 状态 | 数量 |
|------|------|
| ✅ 已加固 | 9 |
| ⚠️ 部分加固 | 6 |
| ⚠️ 脆弱（无外部机制） | 17 |
| **合计** | 32 |

---

## 加固建议

### 可用 hook 加固（投入产出比高）

| 优先级 | 脆弱点# | 建议方案 | hook 类型 |
|--------|---------|----------|-----------|
| P0 | #15 | 结果包六要素检查：PostToolUse(Write) 写入谱系时检查六要素字段 | PostToolUse |
| P1 | #27 | decide 上下文检查：PreToolUse(Bash) 拦截 `gemini_challenger decide` 调用，检查 `--context-file` 参数 | PreToolUse |
| P1 | #8 | spec-write-guard 升级：对 CLAUDE.md/已结算谱系的修改从"警告"升级为"阻断" | PreToolUse |
| P2 | #26 | session 更新检查：Stop hook 中检查 session 文件最后修改时间是否在本轮内 | Stop |

### 可用模板形状加固（无需 hook）

| 脆弱点# | 建议方案 |
|---------|----------|
| #4, #23 | 谱系优先于汇总——在 session 写入守卫中检查：session 更新时，本轮是否有新谱系写入（如有矛盾处理） |
| #24 | definitions/ 写入守卫增加仪式路径检查：检查是否存在 `.chanlun/.ritual-in-progress` 标记文件 |

### 结构性不可加固（需接受为系统特征）

| 脆弱点# | 原因 |
|---------|------|
| #2, #11 | 不绕过矛盾——需要理解代码语义，hook 无法判断"这是 workaround 还是正常实现" |
| #6 | 对象否定对象——需要理解代码语义中的否定来源 |
| #7, #29, #30 | 蜂群默认模式——需要理解任务结构的可分解性，hook 无法判断 |
| #12, #13, #14 | 四分法自检——内部推理过程，hook 无法观测 |
| #16 | 测试失败判断——需要理解定义语义才能区分"定义冲突"和"实现错误" |
| #21 | agent "你不做的事"——需要理解 agent 行为语义（Lead 除外，已有物理约束） |
| #22 | meta-observer 异质审查约束——跨 agent 协调，无法用单点 hook 强制 |
| #25 | 矛盾处理后必须写谱系——"矛盾处理"事件本身不可被 hook 检测 |

---

## 推导链

1. 070号发现：ceremony 后 LLM 停止 = 纯文本指令对抗生成惯性失败
2. 016号发现：知道规则 ≠ 执行规则（spec-execution gap 的原型）
3. 036号结晶：声明-能力一致性缺口检测模式
4. 本扫描将 070/016/036 的模式系统性应用于全部纯文本指令
5. 发现 32 条指令中 17 条完全脆弱、6 条部分加固
6. 结构性不可加固的指令（12 条）揭示了一个边界：**hook 只能约束可观测的外部行为（工具调用、文件写入），不能约束内部推理过程**

## 边界条件

本扫描的结论在以下条件下翻转：
- 如果 LLM 的指令遵从率显著提高（如通过 RLHF 专门训练），结构性不可加固的指令可能变得足够可靠
- 如果引入代码级静态分析工具（如 AST 检查），#6（对象否定对象）可以从"不可加固"变为"可加固"
- 如果引入 agent 行为日志 + 事后审计机制，#21（agent 禁止列表）可以从"不可加固"变为"事后可检测"

## 下游推论

1. **hook 加固的优先级排序已给出**（P0/P1/P2），可直接作为工程任务分派
2. **结构性不可加固 ≠ 无解**——它意味着需要不同层级的解法（如静态分析、事后审计、异质质询），而非更多 hook
3. **spec-execution-gap skill（036号）的适用范围确认**：本扫描是该 skill 的首次系统性应用实例
4. **agent "你不做的事"列表的加固模式已有先例**：Lead 的物理权限剥夺（032号）。其他 agent 可以复制此模式——通过 tools 字段物理限制 agent 的工具集（但当前 Claude Code 的 agent 系统不支持 per-agent 工具限制，这是平台层约束）

## 影响声明

- 新增谱系记录 071
- 不修改任何现有文件
- 为后续 hook 加固工作提供优先级排序和具体方案
