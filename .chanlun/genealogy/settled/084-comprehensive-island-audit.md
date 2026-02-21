---
id: "084"
title: "全面孤岛审计：hooks/agents/skills 声明-能力缺口清单"
type: 矛盾发现
status: 已结算
date: "2026-02-21"
negation_source: heterogeneous
negation_model: gemini-3.1-pro-preview
negation_form: expansion
depends_on: ["075", "082", "083", "016", "043"]
related: ["055", "058", "061", "072", "076", "078", "081"]
negated_by: []
negates: []
provenance: "[新缠论]"
---

# 084 — 全面孤岛审计：体系声明-能力缺口（Gemini 异质质询）

**类型**: 矛盾发现
**状态**: 生成态
**日期**: 2026-02-21
**negation_source**: heterogeneous（Gemini 3.1 Pro Preview 异质质询）
**negation_form**: expansion（重构后的声明能力向多个维度扩张，但实际连接未跟上）
**前置**: 075-structural-to-skill, 082-structural-skill-event-driven-gap, 083-clean-terminate-d-strategy, 016-runtime-enforcement-layer, 043-self-growth-loop
**关联**: 055-double-helix, 058-ceremony-as-swarm-zero, 061-island-audit, 072-hook-enforcement-dual-prerequisite, 076-fractal-execution-gap, 078-break-blocking-loop, 081-swarm-continuity

---

## 背景

用户指出：precompact 功能失效，重构后体系发生了结构变化，老的功能未被扬弃，成为孤岛。

本次质询为全面扫描：20 个 hooks、19 个 agents、12 个 skills，寻找所有孤岛和声明-能力缺口。

## Gemini 推理链摘要

Gemini 未通过工具读取文件（调用方式为 challenge 模式，上下文已预构建），基于预扫描事实提炼出 4 个矛盾点：

1. **权限控制死锁**：`lead-permissions.sh` 的 restrict 模式实际无效——全局 `settings.json` allow 一切，`settings.local.json` 的 deny 数组为空，权限降级是幻觉。
2. **Hook I/O 契约混用**：`hub-node-impact-guard.sh` 输出纯文本（非 JSON），框架忽略；`flow-continuity-guard.sh` 和 `ceremony-completion-guard.sh` 输出 `decision:block` 格式，但 `recursive-guard.sh`（同为 PostToolUse）用 `continue:true + systemMessage` 格式，存在不一致。
3. **自生长回路物理断裂**：`precompact-save.sh` 使用 `python3`（Windows 上不可用）和硬编码 Linux 路径 `/home/user/NewChanlun`，导致 session 文件无法生成，整个 043 号自生长链路在 Windows 上断裂。
4. **拓扑守卫自毁**：`topology-guard.sh` 的 ROOT_NODES 包含不存在的 `src/newchan/skills/manifest.yaml`，BFS 从两个（而非三个）根节点出发，导致孤岛检测产生系统性误报。

## 同质质询（对 Gemini 否定的验证）

### 矛盾点 1：权限幻觉 — **判定成立**

定义回溯：Claude Code 的权限合并规则是 allow 取并集、deny 取并集。`settings.json` allow 列表包含 `Bash(*)`、`Write`、`Edit` 等全量权限，`settings.local.json` 的 deny 数组为空，导致 restrict 模式无任何实际约束效果。

反例构造：如果 restrict 模式真的有效，Lead 在 restrict 模式下应无法执行 `Write` 工具。但当前配置下无任何阻止机制。不存在翻转条件。

**否定成立**。

### 矛盾点 2：Hook 格式混用 — **部分成立，部分误判**

Gemini 断言"Hub-node-impact-guard 输出纯文本导致功能失效"——**成立**。该文件 `echo "⚠ Hub..."` 后 `exit 0`，无 JSON 输出，框架会静默忽略，警告从未显示给用户。

Gemini 断言"flow-continuity-guard 和 ceremony-completion-guard 的 `decision:block` 格式错误"——**部分误判**。

边界条件：Claude Code hooks 文档显示 `decision:block` 是 PreToolUse 的规范格式。PostToolUse 的规范格式是 `continue:true/false + systemMessage`，而 Stop hook 的格式需要返回 `{"continue": false}`。但 `recursive-guard.sh`（同为 PostToolUse）使用 `continue:true + systemMessage`，与 `flow-continuity-guard.sh` 使用 `decision:block` 不一致，说明至少有一个是错误的。

实际后果：格式不一致时，Claude Code 框架的行为取决于实现——可能忽略错误格式的 hook，可能以默认行为处理。这导致功能状态不确定。Gemini 的"致命"判定过强，但"不一致"判定成立。

**否定部分成立**：hub-node 纯文本问题成立；flow-continuity 和 ceremony-completion 的格式不一致问题成立（但严重性评级需要降一档）。

### 矛盾点 3：自生长回路断裂 — **判定成立**

定义回溯：
- `precompact-save.sh` 第 15 行：`python3 -c "..."` — Windows 上 python3 不可用（Windows Store alias，执行失败）
- `precompact-save.sh` 第 18 行：`cd "$cwd" 2>/dev/null || cd /home/user/NewChanlun` — fallback 到 Linux 路径
- 实际执行：python3 失败 → cwd 默认为 `.` → `cd "." 2>/dev/null` 成功（但 cwd 不是项目目录）→ session 写入到错误位置或失败

反例构造：如果 precompact-save.sh 成功，session 文件会存在于 `.chanlun/sessions/`，post-session-pattern-detect.sh 才能读取。当前没有 session 文件，pattern-buffer.yaml 虽然存在（可能曾经在其他环境创建过），但不会被更新。

**否定成立**。整个链路：precompact → session → pattern-buffer → crystallization-guard → skill 结晶，在 Windows 上全部断裂。

### 矛盾点 4：拓扑守卫自毁 — **判定成立**

定义回溯：BFS 三个根节点之一 `src/newchan/skills/manifest.yaml` 不存在，实际上是 `.chanlun/manifest.yaml`。BFS 从不存在的节点出发 → 该节点不在 `all_files` 集合中 → BFS 跳过它 → 所有本应通过该路径可达的文件被判为孤岛（误报）。

反例构造：如果该文件存在，误报消失。但文件不存在，误报持续产生。

**否定成立**。

---

## 全面孤岛清单（结合 Gemini 质询结论）

### Hooks — 逐项状态

| Hook | 注册状态 | 功能状态 | 孤岛判定 | 优先级 |
|------|---------|---------|---------|-------|
| precompact-save.sh | ✅ PreCompact | ❌ 失效（python3 失败 + Linux 路径） | 退化 | P0 |
| session-start-ceremony.sh | ✅ SessionStart | ⚠ 部分工作（依赖 session 文件，但 precompact 失败则无 session） | 退化 | P1 |
| ceremony-guard.sh | ✅ PreToolUse(Task) | ❌ 空壳（直接放行） | 孤岛（明确废弃） | P3 |
| double-helix-verify.sh | ✅ PreToolUse(Bash) | ✅ 功能正常（依赖 GOOGLE_API_KEY + newchan 包） | 正常 | — |
| definition-write-guard.sh | ✅ PreToolUse(Write/Edit) | ✅ 正常（验证+警告，非阻断，符合原则0） | 正常 | — |
| genealogy-write-guard.sh | ✅ PreToolUse(Write/Edit) | ✅ 正常（验证+警告） | 正常 | — |
| spec-write-guard.sh | ✅ PreToolUse(Write/Edit) | ✅ 正常（记录+放行） | 正常 | — |
| hub-node-impact-guard.sh | ✅ PreToolUse(Write/Edit) | ❌ 失效（纯文本输出，框架忽略，从未显示警告） | 退化 | P1 |
| flow-continuity-guard.sh | ✅ PostToolUse(Bash) | ⚠ 格式不确定（decision:block 在 PostToolUse 是否有效不明） | 退化 | P1 |
| crystallization-guard.sh | ✅ PostToolUse(Bash) | ⚠ 部分失效（debt 文件不存在，pattern 路径依赖 session） | 退化 | P1 |
| topology-guard.sh | ✅ PostToolUse(Bash) | ⚠ 误报（ROOT_NODES 路径错误，manifest 检查路径正确） | 退化 | P1 |
| recursive-guard.sh | ✅ PostToolUse(TaskUpdate) | ✅ 正常（格式正确，功能完整） | 正常 | — |
| result-package-guard.sh | ✅ PostToolUse(Write/Edit) | ✅ 正常（验证+警告，符合原则0） | 正常 | — |
| downstream-action-guard.sh | ✅ PostToolUse(Write/Edit) | ✅ 正常（downstream_audit.py 存在） | 正常 | — |
| team-structural-inject.sh | ✅ PostToolUse(TeamCreate) | ✅ 正常（D策略实现，从 dispatch-dag 读取 skill） | 正常 | — |
| ceremony-completion-guard.sh | ✅ Stop | ⚠ 格式不确定（decision:block 在 Stop hook 是否有效不明） | 退化 | P1 |
| meta-observer-guard.sh | ✅ Stop | ❌ 失效（默认 hotfix 模式自动放行，二阶反馈虚设） | 退化 | P2 |
| post-session-pattern-detect.sh | ✅ Stop | ❌ 失效（依赖 session 文件，Windows 上 precompact 失败后无数据） | 退化 | P1 |
| lead-permissions.sh | ❌ 未注册（手动） | ❌ 权限控制无效（deny 为空，全局 allow 覆盖） | 退化 | P0 |
| dag-validation-guard.sh | ❌ 未注册 | ❌ 从未触发（孤岛文件） | 孤岛 | P2 |

### Agents — 逐项状态

所有 19 个 agent 文件存在（.claude/agents/*.md）。在 075号 D策略架构下，agents 不再作为 teammate spawn，而是作为 skill 的指令源。agent 文件本身是知识文档，不需要"注册"——它们在被调用时被读取。

| Agent | 文件存在 | 在 dispatch-dag event_skill_map | 功能状态 |
|-------|---------|-------------------------------|---------|
| genealogist | ✅ | ✅ | 正常（按需读取） |
| quality-guard | ✅ | ✅ | 正常 |
| meta-observer | ✅ | ✅ | 退化（meta-observer-guard 失效，但可手动调用） |
| source-auditor | ✅ | 需验证 | 正常（手动调用） |
| topology-manager | ✅ | ✅ | 正常 |
| gemini-challenger | ✅ | ✅ | 正常 |
| claude-challenger | ✅ | ✅ | 正常 |
| skill-crystallizer | ✅ | 需验证 | 退化（结晶链路断裂） |
| code-verifier | ✅ | 需验证 | 孤岛候选（无事件触发路径） |
| meta-lead | ✅ | ✅ | 正常 |
| ECC agents (9个) | ✅ | N/A | 正常（手动调用） |

### Skills — 逐项状态

所有 6 个 skill 目录下的 SKILL.md 存在。Skills 是知识文档，不需要运行时注册。状态取决于调用路径是否畅通。

| Skill | 文件存在 | 调用路径 |
|-------|---------|---------|
| meta-orchestration | ✅ | 正常 |
| spec-execution-gap | ✅ | 正常 |
| knowledge-crystallization | ✅ | 退化（结晶守卫链路断裂） |
| math-tools | ✅ | 正常 |
| orchestrator-proxy | ✅ | 正常 |
| gemini-math | ✅ | 正常 |

---

## 核心矛盾摘要

**P0 级（影响基础设施，需立即修复）：**

1. `precompact-save.sh` 在 Windows 上完全失效：
   - 将 `python3` 改为 `python`
   - 移除硬编码 Linux fallback 路径
   - 修复后，session → pattern-buffer → crystallization 整个链路才能恢复

2. `lead-permissions.sh` 的权限控制无效：
   - restrict 模式需要在 `deny` 数组中显式列出要禁止的工具
   - 当前 deny 为空 → 全局 allow 完全胜出

**P1 级（功能退化，影响体系自洽性）：**

3. `hub-node-impact-guard.sh` 需改为 JSON 输出
4. `topology-guard.sh` ROOT_NODES 路径错误（`src/newchan/skills/manifest.yaml` → `.chanlun/manifest.yaml`）
5. `flow-continuity-guard.sh` 和 `ceremony-completion-guard.sh` 的格式需与 `recursive-guard.sh` 统一
6. `.chanlun/.crystallization-debt.json` 从未创建，需初始化

**P2 级（体系完整性问题，可暂时接受）：**

7. `meta-observer-guard.sh` 的 hotfix 模式导致二阶反馈虚设——需要重新评估是否启用 STRICT_MODE
8. `dag-validation-guard.sh` 未注册（孤岛）——需要添加到 settings.json 或明确废弃

**P3 级（已知废弃，可清理）：**

9. `ceremony-guard.sh` 完全空壳——可考虑删除或在注释中明确声明为永久占位符

---

## 边界条件

以下条件下，本审计结论会翻转：

1. **如果 Claude Code 的 PostToolUse 支持 `decision:block` 格式**（与 PreToolUse 统一）：flow-continuity-guard 和 ceremony-completion-guard 的格式问题消失，两者功能正常。
2. **如果环境中 `python3` 可用**（如在 Linux/Mac 上）：precompact-save.sh 正常工作，整个自生长链路恢复。
3. **如果 Claude Code 的权限合并策略是 local 完全覆盖 global**（而非并集）：lead-permissions.sh 的 restrict 模式可能有效。

## 下游推论

1. precompact-save.sh 修复后，session → pattern-buffer → crystallization-guard → skill 结晶链路全部恢复（5 个组件联动）
2. topology-guard ROOT_NODES 修复后，孤岛检测结果可信度恢复
3. hub-node-impact-guard JSON 修复后，Hub 节点修改时才会产生实际警告
4. lead-permissions.sh 修复后，032号谱系的"神圣疯狂"（ceremony 后 Lead 权限降级）才能真正实施

## 谱系引用

- **061号**（island-audit）：第一次孤岛扫描，发现拓扑孤岛
- **072号**（hook-enforcement-dual-prerequisite）：hooks 执行层双前提（存在 + 格式正确）
- **082号**（structural-skill-event-driven-gap）：已知的结构 skill 事件驱动孤岛
- **016号**（runtime-enforcement-layer）：规则没有代码强制就不会被执行

## 影响声明

本谱系新增内容：
- 全面 hooks/agents/skills 孤岛清单（20+19+12 个组件逐项）
- 识别 2 个 P0 级基础设施缺陷（precompact python3、权限控制无效）
- 识别 6 个 P1 级功能退化
- 识别 1 个真正的孤岛文件（dag-validation-guard.sh 未注册）
- 确认 043号自生长回路在 Windows 上完全断裂

影响模块：`.claude/hooks/`（8个组件需修复）、`.claude/settings.json`（lead-permissions 配置逻辑）、`.chanlun/`（需创建 crystallization-debt.json 初始文件）
