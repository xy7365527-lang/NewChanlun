---
id: "087"
title: "086号全C重新决断：五问题诚实修复（编排者INTERRUPT）"
type: 选择
status: 已结算
date: "2026-02-21"
negation_source: heterogeneous
negation_model: gemini-3.1-pro-preview
negation_form: expansion
depends_on: ["086", "085", "082", "073b", "075", "041", "018"]
related: ["069", "056", "076", "077", "084", "036"]
negated_by: []
negates: ["086"]
provenance: "[新缠论]"
---

# 087 — 086号全C重新决断：五问题诚实修复

**类型**: 选择（编排者 INTERRUPT 触发的重新决断）
**状态**: 已结算
**日期**: 2026-02-21
**negation_source**: heterogeneous（Gemini 编排者代理，041号）
**negation_model**: gemini-3.1-pro-preview
**negation_form**: expansion（086号全C决断的否定——否定"声明缺口"等同于"修复缺口"）
**前置**: 086-island-repair-strategy-decisions, 085-meta-orchestration-full-audit, 082-structural-skill-event-driven-gap, 073b-trampoline-recursion-approximation, 075-structural-to-skill, 041-orchestrator-proxy, 018-four-way-classification
**关联**: 069-recursive-topology, 056-swarm-recursion-default-mode, 076-fractal-execution-gap, 077-meta-observer-v16b, 084-comprehensive-island-audit, 036-spec-execution-gap-crystallization

---

## 编排者 INTERRUPT 原因

086号全选 C 选项的核心问题：将"文档化缺口"等同于"消除缺口"。
声明-能力缺口的消除方式只有两种：
- **A**：实现能力（能力升级到声明水平）
- **B**：降低声明（声明降级到能力水平）

C 选项（保留声明 + 加注释说明缺口）不消除缺口，只文档化缺口。
这是 036号谱系教训的再现：声明了 X 但实际能力不匹配。

---

## 五个决断（重新 Gemini decide + 同质质询）

### 问题 1：递归从未发生（P0）

**Gemini 决断：选项 B — 降低声明**

**推理链**：
当前系统运行的是 ceremony 链顺序迭代（v12→v21），不是 Trampoline。
Trampoline 要求每轮将新任务写回队列再弹跳——ceremony 链是线性的，每轮从头扫描，不是写回队列再取。
085号审计确认：从未发生子蜂群嵌套，fractal_template 从未实例化。
保留"递归（蜂群→子蜂群→子子蜂群）"声明是虚假声明。诚实优先于理论完整性。

**同质质询判定：成立（含修正）**

- 定义回溯：073b 辨认了 Trampoline 机制，但 085 号审计明确 ceremony 链连 Trampoline 都不是。
- 反例构造：如果 ceremony 确实实现了"任务写回队列→Lead 再弹跳"，则 B 过激。但实际是每次 ceremony 从文件系统头扫描，无队列弹跳机制。反例不成立。
- 修正：Gemini 建议删除 fractal_template 和 recursion_rules——此修正过激。这两个字段是未实现的设计意图，应标注为"设计意图，未实现"而不是删除。删除会丢失架构演进方向。

**执行方案（B 选项精确版）**：
1. 修改 `CLAUDE.md` 原则15：将"递归（蜂群→子蜂群→子子蜂群）"改为"当前实现为扁平 Trampoline（Lead→Worker L1），子蜂群嵌套为设计意图（未实现，见 069/073b 号谱系）"
2. `dispatch-dag.yaml` 的 `fractal_template` 和 `recursion_rules` 保留，添加注释 `# 设计意图，当前未实例化（085号审计）`
3. 原则0中的"所有递归拓扑异步自指蜂群"改为"所有拓扑异步自指蜂群（当前实现；递归为设计意图）"

**边界条件**：Claude Code 平台支持 subagent 嵌套时，立即实现真正调用栈递归，本决断失效。

---

### 问题 2：claude-challenger 完全孤岛（P1）

**Gemini 决断：选项 B — 删除虚假触发声明，改为手动**

**推理链**：
dispatch-dag 声明了 `re_challenge` 和 `stale_generative` 两个自动触发事件。
这两个事件在当前系统中根本不存在（无 hook 产生这些事件）。
如果设计意图是手动调用，声明应如实反映：手动。
虚假的自动触发声明维持了"双向质询是自动发生的"假象，实际上从未发生。

**同质质询判定：成立**

- 定义回溯：082号 D策略允许手动认领模式——手动触发合理。但 dispatch-dag 声明的是自动触发，与 D策略的"hooks 提示"也不符（没有 hook 提示）。声明虚假。
- 反例构造：如果 `re_challenge` 事件确实被某个 hook 产生，则保留声明合理。但 settings.json 和所有 hooks 均无此事件。反例不成立。
- 推论检验：B 选项后，claude-challenger 作为手动 skill 存在，Lead 在发现 Gemini 产出存疑时手动调用。与 082号 D策略同构，一致。

**执行方案（B 选项）**：
修改 `dispatch-dag.yaml` 中 claude-challenger 的 triggers：
- 删除：`event: re_challenge` 和 `event: stale_generative`
- 替换为：`event: manual_invocation, description: "Lead 发现 Gemini 产出存疑时手动调用"`

**边界条件**：系统建立了对"Gemini 决策质量下降"的可观测指标（如连续 3 次 INTERRUPT）时，引入自动提示 hook。

---

### 问题 3：source-auditor 完全孤岛（P1）

**Gemini 决断：选项 A — 实现 docs/** hook 提示**

**推理链**：
dispatch-dag 声明 `file_write(docs/**)` 触发 source-auditor，但 settings.json 无对应 hook。
docs/ 是缠论形式化的核心资产（三级权威链文档），修改后如果不检查溯源标签，可能引入理论漂移。
实现 PostToolUse hook 匹配 docs/** 输出提示文本，成本极低，消除虚假声明。
这符合 082号 D策略（hooks 提示 + Lead 认领），不需要代码层自动调用 agent。

**同质质询判定：成立（含路径修正）**

- Gemini 建议修改 `.vscode/settings.json` — 错误，本项目 hooks 配置在 `.claude/settings.json`
- 定义回溯：082号 D策略明确"hooks 检测文件写入，输出提示文本"是合法的物理实现。与声明对齐。
- 反例构造：如果 docs/** 修改极其罕见，hook 触发开销大于价值——但实际 docs/ 是活跃目录（三级权威链文档在持续补录），不罕见。反例不成立。

**执行方案（A 选项，修正路径）**：
在 `.claude/settings.json` 的 PostToolUse hooks 中添加：
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/hooks/source-auditor-prompt.sh"
          }
        ]
      }
    ]
  }
}
```
新建 `.claude/hooks/source-auditor-prompt.sh`：检测写入路径是否匹配 `docs/**`，若匹配则输出提示。

实际执行：检查 settings.json 结构后选择最小侵入的添加方式（不破坏现有 hook 配置）。

**边界条件**：docs/** 修改频率超过每 session 10 次时，考虑冷却机制（相同路径 5 分钟内只提示一次）。

---

### 问题 4：skill-crystallizer 无事件连接（P2）

**Gemini 原决断：B（降低声明）**
**同质质询后修正：A（实现连接）**

**修正推理链**：
Gemini 判断 precompact 链路断裂，因此建议 B。
但实际核查：
- `.chanlun/sessions/` 存在大量 session 文件（2026-02-17 至今）
- `.chanlun/pattern-buffer.yaml` 存在且有数据（包含 3 个模式，1 个已 promoted）
- precompact-save.sh 现在使用 `python`（非 python3），链路实际工作

结论：precompact 链路已工作。Gemini 的 B 决断基于错误的事实前提。

**实际问题**：pattern-buffer 有数据，但 skill-crystallizer 没有被触发——这不是链路断裂，是事件检测缺失：
- `crystallization-guard.sh` 在 PostToolUse 触发，但检查的是 `.chanlun/.crystallization-debt.json`（不存在）
- pattern-buffer 中 `status: promoted` 的模式需要被检测并触发结晶

**修正决断：A — 实现事件检测连接**

执行方案：
1. 创建 `.chanlun/.crystallization-debt.json`（初始化空文件）：`{"patterns": [], "last_check": null}`
2. 修改 `crystallization-guard.sh`：检查 pattern-buffer.yaml 中是否有 `status: promoted` 的模式，若有则触发提示

**边界条件**：pattern-buffer 长期为空（无 observed 以上状态的模式），skill-crystallizer 静默等待是合理的，不需要强制触发。

---

### 问题 5：meta-observer 被降级（P2）

**Gemini 决断：选项 A — 修复 advisory 输出**

**推理链**：
meta-observer-guard.sh 当前在非严格模式下：自动落标后直接 `exit 0`，无任何输出。
这不是 advisory 模式，而是完全无效模式。
086号决断想要的是"advisory（运行+建议性输出，不阻断）"，但实际代码实现的是"hotfix（自动放行，无输出）"。
修复：在 exit 0 之前添加建议性输出，使 meta-observer 在 session 结束时产生可见的提示。

**同质质询判定：成立**

- 定义回溯：082号 D策略要求"hooks 提示"。当前 hook 无任何提示 = 不符合 D 策略。A 选项修复了 D 策略的物理实现。
- 反例构造：如果增加输出会导致大量噪音，A 不合理。但一次 session 结束只产生一行提示，不是噪音。反例不成立。
- 推论检验：A 选项后，meta-observer 在 session 结束时产生"[Advisory] 二阶观察已跳过，可手动执行"提示，Lead 在上下文中看到并可选择是否执行。与 D 策略同构，一致。

**执行方案（A 选项）**：
修改 `.claude/hooks/meta-observer-guard.sh` 第40-45行附近：
在 `exit 0` 之前添加：
```bash
printf '{"continue": true, "systemMessage": "[meta-observer advisory] 本 session 二阶观察已跳过（STRICT_MODE=0）。如需执行：读取 .claude/agents/meta-observer.md 并对本 session 执行二阶观察。"}\n'
```
（JSON 格式输出，符合 PostToolUse/Stop hook 规范）

**边界条件**：连续 5 次 session 的 advisory 提示均未被认领（可从 .meta-observer-executed 标记缺失观察），升级为 STRICT_MODE 强制提示。

---

## 五个决断汇总

| 问题 | 原 086 决断 | 087 决断 | 变化 |
|------|------------|---------|------|
| 1 递归声明（P0） | C（保留声明+Trampoline注释） | B（降低声明，承认"设计意图未实现"） | C→B |
| 2 claude-challenger（P1） | C（保留触发声明+手动注释） | B（删除虚假触发，改为手动） | C→B（更激进） |
| 3 source-auditor（P1） | C（保留触发声明+手动注释） | A（实现 hook 提示，消除声明缺口） | C→A |
| 4 skill-crystallizer（P2） | C（保留声明+长期工程项） | A（实现事件检测，precompact 链路已工作） | C→A |
| 5 meta-observer（P2） | C→A（修复脚本产生 advisory 输出） | A（同，但更精确的实现规范） | 一致（A） |

---

## 立即执行项

### P0 级（声明诚实化）
1. **CLAUDE.md 原则15 修改**：去掉"递归（蜂群→子蜂群→子子蜂群）"声明，改为"当前实现为扁平 Trampoline，子蜂群嵌套为设计意图（未实现）"
2. **CLAUDE.md 原则0 修改**：去掉"Gemini decide 方案C + 073号谱系"（该注释引用了已被推翻的决断），改为引用 087号

### P1 级（消除虚假声明）
3. **dispatch-dag.yaml 修改**：
   - claude-challenger：删除 `re_challenge` + `stale_generative` 触发，改为 `manual_invocation`
   - source-auditor：保留触发声明（dispatch-dag.yaml 中），因为 A 选项会实现 hook
4. **新建 `.claude/hooks/source-auditor-prompt.sh`**
5. **修改 `.claude/settings.json`**（添加 PostToolUse docs/** hook）

### P1 级（修复 advisory 输出）
6. **修改 `.claude/hooks/meta-observer-guard.sh`**：advisory 模式产生 JSON 格式提示

### P2 级（实现结晶事件连接）
7. **初始化 `.chanlun/.crystallization-debt.json`**
8. **修改 `.claude/hooks/crystallization-guard.sh`**：检查 pattern-buffer 中的 promoted 模式

---

## 边界条件汇总

| 翻转条件 | 受影响决断 | 触发动作 |
|---------|---------|---------|
| Claude Code 支持 subagent 嵌套 | 问题1 | 实现真调用栈递归，恢复原则15声明 |
| 建立 Gemini 决策质量可观测指标 | 问题2 | 引入自动 hook 提示 claude-challenger |
| docs/** 修改频率超过 10 次/session | 问题3 | 增加冷却机制 |
| pattern-buffer 长期无 promoted 模式 | 问题4 | skill-crystallizer 继续静默等待，A 选项有效 |
| 连续 5 次 meta-observer advisory 未认领 | 问题5 | 升级为 STRICT_MODE |

---

## 同质质询总结

| 问题 | 原判定 | 修正后判定 | 修正原因 |
|------|--------|----------|---------|
| 1 | C 成立 | B 成立 | ceremony 链不是 Trampoline，声明必须降级 |
| 2 | C 成立 | B 成立 | 虚假事件声明必须删除而非注释 |
| 3 | C 成立 | A 成立 | hook 实现成本低，消除缺口优于文档化缺口 |
| 4 | C 成立 | A 成立（事实前提修正） | precompact 链路已工作，应实现连接而非降级声明 |
| 5 | C 成立 | A 成立（已对齐） | advisory 本就需要产生实际输出 |

---

## 谱系链接

- **否定**：086号（全C决断被本谱系推翻）
- **前置**：085号（量化缺口）、082号（D策略基础）、073b号（Trampoline辨认）
- **关联**：036号（声明-能力缺口教训）、069号（递归拓扑理想方案）
- **被本谱系约束的后续动作**：
  1. CLAUDE.md 原则修改（立即执行）
  2. dispatch-dag.yaml 修改（立即执行）
  3. hooks 修改（立即执行）

---

## 影响声明

**本谱系**：
- 五个孤岛修复的重新决断（2B + 2A + 1A）
- 同质质询对 Gemini 决断的修正（问题4 事实前提修正）
- 执行动作清单

**涉及文件（将立即修改）**：
- `CLAUDE.md`（原则0 + 原则15）
- `.chanlun/dispatch-dag.yaml`（claude-challenger 触发方式）
- `.claude/hooks/meta-observer-guard.sh`（advisory 输出）
- `.claude/hooks/source-auditor-prompt.sh`（新建）
- `.claude/settings.json`（添加 PostToolUse hook）
- `.chanlun/.crystallization-debt.json`（初始化）
- `.claude/hooks/crystallization-guard.sh`（检查 promoted 模式）
