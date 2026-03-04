# Lead 串行残余诊断报告

**模式**: diagnose（严格诊断）
**日期**: 2026-02-26
**执行者**: codex-reviewer（代理，非 Codex 本身）
**对象**: team lead（CC 编排者代理）在 RTAS 循环中的串行残余

---

## 1. 诊断结论

Lead 的 RTAS 循环存在 **5 个串行瓶颈点**，其中 3 个可并行化，2 个是结构性必须串行。

最严重的瓶颈：`ceremony_scan.py` 内嵌 pytest 调用（最长阻塞 120 秒），以及 `post-commit-flow.md` 规则在 commit 完成后强制插入串行检查点。

---

## 2. RTAS 循环当前执行顺序

从代码和规则推导出的实际执行链：

```
commit → push → gangju_analysis → ceremony_scan → spawn workstations
                                    └─ [内嵌 pytest, 最长 120s]
                                                 ↓
                                    workstations 并行执行
                                                 ↓
                                    lead 轮询 TaskList（串行等待）
                                                 ↓
                                    review 节点（lead 执行）
                                                 ↓
                                    异质审计节点
                                                 ↓
                                    结晶节点
```

---

## 3. 各瓶颈点分析

### 3.1 ceremony_scan → gangju_analysis 顺序

**当前状态**: 串行（gangju_analysis 先，ceremony_scan 后）

**依据**: `gangju_analysis.py` 文档字符串明确声明：
> "RTAS 循环步骤 7.5：commit+push 之后、ceremony_scan 之前执行"

**是否可并行**: **不可以**。

**根因**: `gangju_analysis.py` 的 `generate_pending_skeleton()` 在 `audit_needed=true` 时会写入 `.chanlun/genealogy/pending/` 新文件。`ceremony_scan.py` 的 `get_session_workstations()` 会扫描 `pending/*.md` 并将其转化为 P0 工位。

这是真实的数据依赖：gangju 的写入是 ceremony_scan 的输入。并行执行会导致 ceremony_scan 漏掉 gangju 刚生成的 pending 谱系工位。

**判定**: 必须串行，依据充分。

---

### 3.2 ceremony_scan 内嵌 pytest（最严重瓶颈）

**当前状态**: 串行阻塞，最长 120 秒

**代码位置**: `ceremony_scan.py:482-508`，`discover_business_tasks()` 函数：

```python
result = subprocess.run(
    [sys.executable, "-m", "pytest", ...],
    cwd=root, capture_output=True, text=True, timeout=120,
)
```

**是否可并行**: **可以**，且应该重构。

**根因**: pytest 是独立的验证操作，与 roadmap 扫描、session 扫描、谱系扫描完全无数据依赖。当前实现将其嵌入 ceremony_scan 主流程，导致整个扫描被阻塞。

**修复方向**: 将测试验证提取为独立工位（`tdd-guide` agent），由 lead 在 spawn 阶段并行分派，而非在 ceremony_scan 内同步执行。

**影响**: 每次 RTAS 循环节省最多 120 秒。

---

### 3.3 commit → push → ceremony_scan 顺序

**当前状态**: 完全串行（commit → push → ceremony_scan）

**是否可并行**: **部分可以**。

**分析**:
- `commit → push`：git 硬依赖，必须串行。
- `push → ceremony_scan`：**可以并行**。

`ceremony_scan.py` 读取的全部是本地文件（roadmap.yaml、session 文件、genealogy/、block-topology/）。它不依赖远程仓库状态。push 完成与否不影响 ceremony_scan 的输入。

**修复方向**: commit 完成后，push 和 ceremony_scan 可以同时启动。

**注意**: `post-commit-flow.md` 规则要求"commit/push 后，在同一个输出中完成总结和下一步行动"。这条规则的意图是防止 RLHF 停顿点，但其字面表述将 push 和 ceremony_scan 都放在 commit 之后的同一个串行链中。规则本身没有禁止 push 与 ceremony_scan 并行——它只要求总结和下一步行动在同一输出中，不要求 push 完成后才能启动 ceremony_scan。

---

### 3.4 spawn 工位后的 monitor 循环

**当前状态**: lead 轮询 TaskList，等待所有工位完成后才进入 review 节点

**是否可并行**: **不可以**（结构性约束）。

**根因**: `sub-swarm-ceremony/SKILL.md` 明确定义了 DAG 依赖：
> "任务节点 → 审查节点（任务完成后才能审查）"

review 节点（t 时刻审查 t-1 产出）是异步自指的核心实现（约束3：执行不可自观性）。lead 必须等工位产出物化后才能审查。这不是实现缺陷，是五约束框架的结构性要求。

**可优化点**: 当前 lead 等待**所有**工位完成后才开始 review。如果工位之间无依赖，lead 可以在**每个工位完成时**立即对该工位执行 review，而不是等全部完成。这将 review 从"批量串行"变为"流式并行"。

---

### 3.5 测试验证作为 lead 自执行操作

**当前状态**: lead 在 ceremony_scan 内直接执行 pytest

**是否可并行**: **可以**，且违反了蜂群架构原则。

**根因**: `swarm-architecture.md` 原则 C（递归节点默认行为）：
> "语法/编译/Lint 级错误的修复流程启动（分派给 teammates，当前节点不自行执行）"

pytest 执行属于验证操作，应分派给 `tdd-guide` teammate，而非 lead 自行执行。当前实现违反了"lead 不自行执行工位级操作"的架构原则。

---

### 3.6 post-commit-flow.md 隐式串行约束

**规则文本**:
> "commit/push 后，在同一个输出中完成总结和下一步行动——不允许'先总结，再决定下一步'的两步模式"

**隐式串行效果**: 每次 commit 完成后，lead 必须在同一输出中声明下一步并立即执行工具调用。这在 commit 完成点插入了一个强制检查点，使 lead 无法在 commit 进行中预先准备下一步操作。

**是否可消除**: **不可以**（规则意图是防止 RLHF 停顿点，消除会引入新的停顿风险）。

**可优化点**: 规则要求"在同一输出中"，但不要求 push 完成后才能声明下一步。lead 可以在 commit 完成后立即声明"push + ceremony_scan 并行启动"，而不是等 push 完成后再声明 ceremony_scan。

---

## 4. 可并行化 vs 必须串行清单

### 可并行化

| 操作对 | 当前状态 | 并行化依据 | 预期收益 |
|--------|----------|-----------|---------|
| push ‖ ceremony_scan | 串行 | ceremony_scan 只读本地文件，不依赖远程状态 | 节省 push 网络延迟 |
| pytest 验证 → 独立工位 | lead 内嵌执行 | pytest 与其他扫描无数据依赖；架构原则要求分派给 teammate | 节省最多 120s |
| ceremony_scan 内部子扫描 | 单进程顺序执行 | roadmap 扫描、session 扫描、谱系异常检测、downstream_audit、async_self_reference 互相无数据依赖 | 减少扫描总耗时 |
| 工位完成后流式 review | 批量等待后 review | 各工位产出独立，无需等全部完成 | 减少 review 等待时间 |

### 必须串行

| 操作链 | 串行原因 | 依据 |
|--------|---------|------|
| gangju_analysis → ceremony_scan | gangju 写 pending 谱系，ceremony_scan 读 pending 谱系 | 数据依赖（写→读） |
| commit → push | git 硬依赖 | git 协议 |
| workstations 完成 → review 节点 | 异步自指：t 审查 t-1，产出必须物化后才能审查 | 五约束框架约束3 |
| review → 异质审计 → 结晶 | DAG 依赖边（sub-swarm-ceremony SKILL.md 明确定义） | 097号 DAG 模板 |

---

## 5. 根因定位

**主要根因**（按严重性排序）：

1. **pytest 内嵌于 ceremony_scan**（`ceremony_scan.py:482-508`）：最严重，阻塞最长 120s，且违反架构原则（lead 不应自执行工位级操作）。

2. **push 与 ceremony_scan 未并行**：push 是网络 I/O，ceremony_scan 是本地文件读取，两者完全独立，当前串行执行浪费了 push 的等待时间。

3. **ceremony_scan 内部子扫描未并行化**：`downstream_audit`、`async_self_reference`、`detect_genealogy_anomalies` 等子扫描在单进程中顺序执行，均为只读操作，可并行。

---

## 6. 边界条件

- 如果 gangju_analysis 不生成 pending 谱系（`audit_needed=false`），则 gangju 和 ceremony_scan 之间无数据依赖，可并行。当前代码无此优化路径。
- 如果 ceremony_scan 的 `discover_business_tasks` 被提取为独立工位，需要确保 ceremony_scan 不再触发 pytest，否则会重复执行。
- 流式 review（工位完成即 review）要求 lead 能区分"部分完成"和"全部完成"状态，当前 TaskList 轮询机制支持此判断。

---

## 7. 影响声明

- 涉及文件：`scripts/ceremony_scan.py`（`discover_business_tasks` 函数）、`scripts/gangju_analysis.py`（执行顺序）
- 涉及规则：`.claude/rules/post-commit-flow.md`（隐式串行约束）
- 涉及架构：`sub-swarm-ceremony/SKILL.md`（DAG 依赖定义）、`swarm-architecture/SKILL.md`（原则C：lead 不自执行工位级操作）
- 不涉及定义层变更，不需要走 `/escalate`
