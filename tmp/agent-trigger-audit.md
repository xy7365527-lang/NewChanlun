# Agent 触发状态完整审计

**审计方法**：异质视角——从能力层反查声明层，对每个 agent 检查：
1. `.md` 文件声称的触发条件
2. `settings.json` 中是否有对应 hook
3. `ceremony_scan.py` 中是否有消费逻辑
4. `dispatch-dag.yaml` 中的位置和触发定义

## 一、触发状态总表

### event_skill_map 中的结构工位（structural）

| Agent | 声称触发条件 | Hook 实现 | ceremony_scan 消费 | 实际触发路径 | 状态 |
|-------|------------|----------|-------------------|-------------|------|
| genealogist | task_complete, /escalate, session_end | 无直接 hook | 无直接消费 | Lead 手动认领（D策略082号） | **缺口**：无 hook 产生 task_complete 事件 |
| quality-guard | file_write(.chanlun/genealogy/**), file_write(src/**), /inquire | post-write-edit-dispatcher.sh 内含 result-package-guard 逻辑 | 无直接消费 | hook 内联 + Lead 手动认领 | **部分实现**：result-package-guard 是 hook 内联检查，不 spawn agent |
| meta-observer | session_end, swarm_cycle_end | meta-observer-guard.sh（Stop hook） | 无直接消费 | Stop hook 提醒 + Lead 认领 | **部分实现**：Stop hook 提醒但不 spawn agent |
| code-verifier | file_write(src/**/*.py), file_write(tests/**/*.py) | 无直接 hook | 无直接消费 | 无自动触发路径 | **缺口**：声明了文件变更触发但无 hook 实现 |
| topology-mutator | genealogy_settlement（含 negates/topo_effect） | topology-mutator-prompt.sh（via post-write-edit-dispatcher） | detect_pending_topo_effects() 扫描 | hook 提醒 + Lead 认领 | **部分实现**：hook 提醒存在，ceremony_scan 有扫描 |

### event_skill_map 中的条件触发工位（conditional）

| Agent | 声称触发条件 | Hook 实现 | ceremony_scan 消费 | 实际触发路径 | 状态 |
|-------|------------|----------|-------------------|-------------|------|
| gemini-challenger | /challenge, spec/theorems/* 变更, escalate_choice, genealogy_settlement | 无直接 hook | 无直接消费 | Lead 手动调用 CLI | **缺口**：4个声称触发中只有手动调用可执行 |
| claude-challenger | manual_invocation | 无 hook | 无消费 | Lead 手动 spawn | **一致**：声称手动，实际手动 |
| codex-challenger | /code-review, test_failure, plan_review, manual | 无直接 hook | 无消费 | Lead 手动 spawn / /code-review 命令文档中声称触发 | **部分缺口**：test_failure 自动触发无实现 |
| topology-analyst | swarm_cycle_end（区块>0）, manual | 无 hook | 无消费 | Lead 手动 spawn | **缺口**：swarm_cycle_end 自动触发无实现 |
| source-auditor | file_write(docs/**) | 无直接 hook（source-auditor-prompt.sh 存在但未注册到 settings.json） | 无消费 | 无自动触发路径 | **缺口**：source-auditor-prompt.sh 存在但未注册 |
| topology-manager | genealogy_count_threshold（>30 或 pending>5） | 无 hook | 无消费 | 无自动触发路径 | **缺口**：声称的阈值触发无任何实现 |
| skill-crystallizer | pattern_buffer_ready | crystallization-guard.sh（PostToolUse/Bash） | get_pattern_buffer_candidate_count() | hook 警告 + ceremony_scan 生成工位 | **部分实现**：有检测但不自动 spawn agent |

### platform_layer 工位（ECC 工程底座）

| Agent | 声称触发条件 | Hook 实现 | ceremony_scan 消费 | 实际触发路径 | 状态 |
|-------|------------|----------|-------------------|-------------|------|
| architect | 按需调用 | 无 | 无 | 由 /plan 或 Lead 手动 spawn | **一致**：按需调用 |
| planner | 按需调用（/plan） | 无 | 无 | /plan 命令触发 | **一致** |
| tdd-guide | 按需调用（/tdd） | 无 | 无 | /tdd 命令触发 | **一致** |
| code-reviewer | 按需调用（/code-review） | 无 | 无 | /code-review 命令触发 | **一致** |
| python-reviewer | 按需调用 | 无 | 无 | 由 code-reviewer 或 Lead 手动 | **一致** |
| security-reviewer | 按需调用 | 无 | 无 | Lead 手动 | **一致** |
| refactor-cleaner | 按需调用（/refactor-clean） | 无 | 无 | /refactor-clean 命令触发 | **一致** |
| doc-updater | 按需调用（/update-docs） | 无 | 无 | /update-docs 命令触发 | **一致** |
| meta-lead | 蜂群中断路由器 | 无 | 无 | 作为 Lead 角色运行（不是被 spawn 的 agent） | **一致** |
| build-error-resolver | build_failure（连续>=3） | 无 hook | 无消费 | 由 /build-fix 命令或 Lead 手动 | **部分缺口**：声称自动触发但无实现 |

## 二、缺口汇总

### 严重缺口（声明了自动触发条件但完全无实现）

1. **code-verifier**：声明了 `file_write(src/**/*.py)` 和 `file_write(tests/**/*.py)` 触发，但 settings.json 中无对应 hook，ceremony_scan 中无消费逻辑。这个 agent 从未被自动触发过。

2. **topology-manager**：声明了 `genealogy_count_threshold(>30 或 pending>5)` 触发，但无 hook、无 ceremony_scan 消费。当前谱系已 350+ 条，远超阈值，但此 agent 从未被自动触发。

3. **source-auditor**：声明了 `file_write(docs/**)` 触发。`source-auditor-prompt.sh` 文件存在于 `.claude/hooks/` 目录但**未注册到 settings.json**。结果：hook 存在但永远不会被执行。

4. **topology-analyst**：声明了 `swarm_cycle_end` 触发，但无 hook 实现。只有 Lead 手动 spawn 才能激活。

### 中等缺口（部分实现但不完整）

5. **gemini-challenger** 的 4 个触发条件中，只有 manual_invocation 可执行：
   - `/challenge` 命令存在且可手动调用 ✓
   - `spec/theorems/*` 变更 → 无 hook 监听此路径
   - `escalate_choice` → 无自动路由到 Gemini decide
   - `genealogy_settlement` → 无 hook 检测谱系结算后自动触发 Gemini 质询

6. **codex-challenger** 的 `test_failure` 自动触发无实现——测试失败时不会自动 spawn Codex diagnose。

7. **quality-guard** 的 agent 角色被 hook 内联逻辑替代：result-package-guard 在 post-write-edit-dispatcher.sh 内作为 Python 代码执行，不 spawn quality-guard agent。

8. **genealogist** 的 `task_complete` 事件无 hook 产生——业务工位完成后不会自动触发谱系检查。

9. **skill-crystallizer** 的 `pattern_buffer_ready` 触发：crystallization-guard.sh 能检测到 candidate 模式并警告，ceremony_scan 能生成工位，但不自动 spawn agent。

### 设计意图确认（D策略 082号）

dispatch-dag.yaml 中多处标注"D策略082号：hooks 提示 + Lead 认领"。这意味着设计意图是**不自动 spawn agent，而是通过 hook 提示 Lead 手动认领**。

但问题是：
- 大部分 hook 提示**并不存在**——settings.json 中只注册了有限的 hook
- Lead（ceremony.md）白名单中没有"检查 hook 提示并认领对应 skill"的步骤
- ceremony_scan 输出的 `required_skills` 字段在 Lead 的消费序列中**无消费者**

## 三、required_skills 字段消费状态

`ceremony_scan.py` 的 `get_required_skills()` 从 dispatch-dag.yaml 的 event_skill_map 读取 structural skill 列表，输出到 JSON 的 `required_skills` 字段。

**消费链断裂**：
1. `ceremony_scan.py` 输出包含 `required_skills`
2. `ceremony.md`（Lead 指令）中无任何步骤消费 `required_skills`
3. Lead 只消费 `workstations` 字段来 spawn 工位
4. `required_skills` 字段被输出后无人读取

结论：`required_skills` 是死数据——产出了但无消费者。

## 四、Hook 文件存在但未注册

以下 hook 文件存在于 `.claude/hooks/` 目录但未在 `settings.json` 中注册：

| Hook 文件 | 声称功能 | 注册状态 |
|-----------|---------|---------|
| source-auditor-prompt.sh | 文档写入时提示 source-auditor | **未注册** |
| ceremony-step-guard.sh | ceremony 步骤检查 | **未注册** |

这些 hook 永远不会被 Claude Code 平台执行。
