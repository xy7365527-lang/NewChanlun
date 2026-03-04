# Spec-Execution Gap 全面审计报告

**审计日期**: 2026-03-04
**审计范围**: CLAUDE.md、ceremony.md、settings.json hooks、gangmu.yaml、scripts/、谱系下游推论
**审计工位**: v155-swarm/spec-gap-audit

---

## 审计结果总表

| # | 位置 | 声明内容 | 实际状态 | 严重程度 | 修复建议 |
|---|------|---------|---------|---------|---------|
| 1 | `.claude/commands/ceremony.md:8` | `ceremony_scan.py --phase initial` | ceremony_scan.py 无 `--phase` 参数（344号已记录：被 v151 移除，声明未同步） | **CRITICAL** | 将 `--phase initial` 从 ceremony.md 步骤1中移除 |
| 2 | `.claude/rules/post-commit-flow.md:23` | `python scripts/ceremony_scan.py --phase rescan` | 同上，`--phase` 已不存在 | **CRITICAL** | 将 `--phase rescan` 改为 `python scripts/ceremony_scan.py` |
| 3 | `.claude/rules/post-commit-flow.md:38` | `ceremony_scan.py --phase rescan 执行完成后` | 同上 | **HIGH** | 文档措辞更新：移除 `--phase rescan` |
| 4 | `.claude/hooks/flow-continuity-guard.sh:66` | hook 注入消息包含 `python scripts/ceremony_scan.py --phase rescan` | hook 指导 LLM 执行不存在的参数，会导致 argparse 报错 | **CRITICAL** | 修改 flow-continuity-guard.sh 第66行，移除 `--phase rescan` |
| 5 | `.claude/hooks/ceremony-step-guard.sh` | 文件存在于磁盘但未注册于 settings.json | 已有注释说明"从 settings.json 移除"（228-4）——设计意图，非缺口 | **LOW** | 无需修复（文档性保留，谱系记录完整） |
| 6-15 | `.claude/hooks/{10个未注册hook}` | hook 存在但未注册于 settings.json | 不活跃——无调用路径 | **LOW** | 确认是否需要注册或可清理（见下方详表） |
| 16 | `ceremony_scan.py:546-548` | 声明"只读扫描"（ceremony.md 不变量第5条） | review-results 写入 consumed 标记 | **MEDIUM** | 要么更新不变量声明，要么将 consumed 标记移到扫描外部 |
| 17 | `.chanlun/gangmu.yaml:21` | `test_pass: <pattern> — 匹配模式的测试文件存在且通过` | ceremony_scan.py `_check_completion` 只检查文件存在，不执行测试 | **MEDIUM** | 要么更新注释为"文件存在即满足"，要么增强 _check_completion 实现 |

---

## 分区详述

### 1. CLAUDE.md 声明审计

**命令（7个）**: 全部有对应的 `.claude/commands/*.md` 文件。

| 命令 | 文件 | 存在 |
|------|------|------|
| /ceremony | `.claude/commands/ceremony.md` | YES |
| /inquire | `.claude/commands/inquire.md` | YES |
| /escalate | `.claude/commands/escalate.md` | YES |
| /ritual | `.claude/commands/ritual.md` | YES |
| /plan | `.claude/commands/plan.md` | YES |
| /tdd | `.claude/commands/tdd.md` | YES |
| /code-review | `.claude/commands/code-review.md` | YES |

**Skill（14个）**: 全部有对应的 `.claude/skills/*/SKILL.md` 文件。

| Skill | 路径 | SKILL.md 存在 |
|-------|------|--------------|
| core-principles | `.claude/skills/core-principles/` | YES |
| domain-conventions | `.claude/skills/domain-conventions/` | YES |
| domain-principles | `.claude/skills/domain-principles/` | YES |
| swarm-architecture | `.claude/skills/swarm-architecture/` | YES |
| project-topology | `.claude/skills/project-topology/` | YES |
| meta-orchestration | `.claude/skills/meta-orchestration/` | YES |
| orchestrator-proxy | `.claude/skills/orchestrator-proxy/` | YES |
| sub-swarm-ceremony | `.claude/skills/sub-swarm-ceremony/` | YES |
| knowledge-crystallization | `.claude/skills/knowledge-crystallization/` | YES |
| spec-execution-gap | `.claude/skills/spec-execution-gap/` | YES |
| math-tools | `.claude/skills/math-tools/` | YES |
| gemini-math | `.claude/skills/gemini-math/` | YES |
| plan-review | `.claude/skills/plan-review/` | YES |
| consensus-ceremony-trigger | `.claude/skills/consensus-ceremony-trigger/` | YES |

**缠论资料路径（5个）**: 全部存在。

| 路径 | 存在 |
|------|------|
| `缠论知识库.md` | YES |
| `docs/chanlun/text/blog/INDEX.md` | YES |
| `docs/chanlun/text/chan99/INDEX.md` | YES |
| `docs/chanlun/text/mindmaps/INDEX.md` | YES |
| `docs/chanlun/README.md` | YES |

**结论**: CLAUDE.md 无 CRITICAL/HIGH 缺口。

### 2. ceremony.md 声明审计

**CRITICAL 发现: `--phase` 幽灵参数**

ceremony.md 步骤1声明:
```
python scripts/ceremony_scan.py --phase initial
```

但 `ceremony_scan.py` 的 argparse 只接受 `--skills`、`--structural`、`--workstations`。`--phase` 参数在 v151-swarm 的 encounter-record 工位修改中被移除（344号谱系记录），但以下位置未同步更新:

1. `ceremony.md:8` — `--phase initial`
2. `post-commit-flow.md:23` — `--phase rescan`
3. `post-commit-flow.md:38` — `--phase rescan`
4. `flow-continuity-guard.sh:66` — hook 注入消息包含 `--phase rescan`

**影响路径**: flow-continuity-guard.sh 会在 git push 成功后注入 block 消息，要求 LLM 执行 `ceremony_scan.py --phase rescan`。如果 LLM 严格遵循 hook 指令，会导致 `argparse` 报 `unrecognized arguments` 错误。实际上 ceremony_push_and_rescan.sh 中的调用已正确更新（不带 `--phase`），但 hook 的误导性指令在非原子链路径中仍然存在。

**不变量声明审计**:

| 不变量 | 声明 | 实际 | 状态 |
|--------|------|------|------|
| 确定性 | 相同文件系统 → 相同输出 | ceremony_scan.py 是确定性的 | OK |
| 并行默认 | 无依赖工位并行 spawn | LLM 层约束，无 hook 强制 | OK |
| 增量持久化 | 每条 completion 写 session | session_append.sh 实现 | OK |
| 不动点终止 | rescan 输出与上轮相同 → 终止 | ceremony_push_and_rescan.sh 输出 JSON 供 Lead 解析 | OK |
| 只读扫描 | scan 不写文件 | review-results consumed 标记写入（行546-548） | **MEDIUM 偏差** |
| 持久化不变量 | 每条退出路径以 session+commit+push 结束 | ceremony_push_and_rescan.sh 原子链实现 | OK |

**"绝对禁止"列表**: 6条禁止项均无自动化检测机制，依赖 LLM 内化。这是设计意图（057号：LLM 不是状态机），不构成缺口。

### 3. settings.json hooks 审计

**已注册 hooks（20条注册，全部文件存在）**: 一致。

**未注册但磁盘上存在的 hooks（11个）**:

| Script | 状态说明 |
|--------|---------|
| ceremony-step-guard.sh | 228-4 明确注释"从 settings.json 移除"，保留为谱系文档 |
| dag-validation-guard.sh | 无注册，无调用路径 |
| definition-write-guard.sh | 无注册，无调用路径 |
| downstream-action-guard.sh | 无注册，无调用路径 |
| genealogy-gemini-verify.sh | 无注册，无调用路径 |
| genealogy-write-guard.sh | 无注册，无调用路径 |
| hub-node-impact-guard.sh | 无注册，无调用路径 |
| result-package-guard.sh | 无注册，无调用路径 |
| source-auditor-prompt.sh | 无注册，无调用路径 |
| spec-write-guard.sh | 无注册，无调用路径 |
| topology-mutator-prompt.sh | 无注册，无调用路径 |

**分析**: `ceremony-step-guard.sh` 有完整谱系解释。其余10个需要确认是待注册还是可清理。不构成声明-能力缺口（无处声称这些 hook 是活跃的）。

### 4. gangmu.yaml 声明审计

**completion_check 引用验证**:

| 目 | check type | 路径/关键词 | 文件存在 | blocked_by | 结论 |
|----|-----------|------------|---------|-----------|------|
| G1-position-management | file_exists | `scripts/position_manager.py` | YES | null | check 通过 |
| G2-fugue-state-machine | file_exists | `src/newchan/fugue_engine.py` | NO | 有阻塞 | 正常——被阻塞 |
| I1-xiaozhuan-da | test_pass | `tests/test_xiaozhuan_da_integration.py` | YES | null | check 通过（但只检查存在） |
| k4-monitor-vps-deploy | script_exists | `deploy/k4-monitor/setup.sh` | YES | null | check 通过 |
| k4-config-encoder | file_exists | `src/newchan/k4_config.py` | YES | null | check 通过 |
| fourth-regime-id | genealogy_settled | keyword "fourth-regime" | N/A | 有阻塞 | 正常——被阻塞 |
| encounter-record | genealogy_settled | keyword "encounter-record" | N/A | null | 待检查 |

**MEDIUM 发现: test_pass 语义偏差**

gangmu.yaml 第21行注释声明:
```
test_pass: <pattern> — 匹配模式的测试文件存在且通过
```

但 ceremony_scan.py 的 `_check_completion()` 对 `test_pass` 只检查文件存在:
```python
if check_type == "test_pass":
    pattern = check.get("pattern", "")
    return os.path.isfile(os.path.join(root, pattern))
```

声明说"存在**且通过**"，实现只检查"存在"。

### 5. scripts/ 工具脚本审计

| 脚本 | 声明的接口 | 实际接口 | 一致性 |
|------|-----------|---------|--------|
| ceremony_scan.py | `--phase` (ceremony.md) | 不支持 `--phase` | **CRITICAL 不一致** |
| ceremony_scan.py | `--skills` / `--workstations` | 支持 | OK |
| session_update.py | `--append` / `--finalize` / 无参数 | 支持 | OK |
| ceremony_push_and_rescan.sh | 原子链 commit→push→rescan | 实现一致 | OK |
| ceremony_state.py | write/read/clear/check + WAL 操作 | 实现一致 | OK |
| session_append.sh | 包装 session_update.py --append | 实现一致 | OK |

### 6. 谱系下游推论审计

**227号 观察1（ceremony_state.py 集成缺口）**:
- 声明: status: resolved
- 验证: ceremony.md 步骤1调用 `ceremony_state.py write 1 initial`，ceremony_push_and_rescan.sh 步骤7/8写入，步骤2/10调用 clear
- 结论: **resolved 属实**

**344号（v151 --phase 移除）**:
- 记录: `ceremony_scan.py 的 --phase 参数被移除——ceremony_push_and_rescan.sh 需要同步更新`
- ceremony_push_and_rescan.sh: 已更新（不带 `--phase`）
- ceremony.md / post-commit-flow.md / flow-continuity-guard.sh: **未更新**
- 结论: **部分 resolved——脚本修复了，声明层3处未同步**

---

## 严重程度统计

| 严重程度 | 数量 | 说明 |
|---------|------|------|
| **CRITICAL** | 3 | ceremony.md `--phase initial`、post-commit-flow.md `--phase rescan`、flow-continuity-guard.sh `--phase rescan` |
| **HIGH** | 1 | post-commit-flow.md 文档描述包含 `--phase rescan` |
| **MEDIUM** | 2 | ceremony_scan.py 只读声明偏差、gangmu.yaml test_pass 语义偏差 |
| **LOW** | 11 | 11个未注册但存在的 hook 文件 |
| **OK** | 26+ | 命令、skill、资料路径、gangmu 文件引用、227号 resolved 下游推论 |

## 根因分析

CRITICAL 缺口的共同根因: **344号谱系记录的代码变更（v151 移除 `--phase`）在 ceremony_push_and_rescan.sh 上做了同步，但在声明层（ceremony.md、规则文件、hook 注入消息）上未同步**。这是036号模式（声明-能力一致性缺口）的传播不完整实例。

---

## 结果包六要素

1. **结论**: 3个 CRITICAL + 1个 HIGH + 2个 MEDIUM 缺口。CRITICAL 全部围绕 `--phase` 幽灵参数——344号代码变更未完整传播到声明层
2. **定义依据**: 036号谱系（声明-能力一致性原则）定义了声明与实现不匹配为缺口；344号谱系记录了 `--phase` 移除事件
3. **边界条件**: 如果 ceremony_scan.py 重新引入 `--phase` 参数（不太可能），CRITICAL 发现会翻转。如果 gangmu.yaml test_pass 的注释改为"文件存在"，MEDIUM 发现会消除
4. **下游推论**: (a) flow-continuity-guard.sh 的误导性指令在非原子链路径中会导致 ceremony 中断（LLM 尝试执行不存在的参数 → argparse 报错 → ceremony 卡住）; (b) gangmu.yaml test_pass 语义偏差意味着 I1-xiaozhuan-da 可能被误判为已完成（文件存在但测试可能失败）
5. **谱系引用**: 036号（声明-能力一致性）、227号（ceremony_state 集成缺口 resolved）、344号（--phase 移除，传播不完整）
6. **影响声明**: 本审计不修改任何代码。需要更新的文件: `.claude/commands/ceremony.md`、`.claude/rules/post-commit-flow.md`、`.claude/hooks/flow-continuity-guard.sh`
