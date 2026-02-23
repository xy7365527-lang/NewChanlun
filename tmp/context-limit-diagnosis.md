# Context Limit 根因诊断报告

日期：2026-02-23
工位：ctx-diag (v151-swarm)

---

## 1. 数据：各组件精确尺寸

### 1.1 每个会话自动注入的静态内容

| 组件 | 字节 | KB | 估计 token |
|------|-----:|---:|----------:|
| CLAUDE.md | 5,487 | 5.4 | ~1,830 |
| 项目 rules（.claude/rules/ 19文件） | 24,379 | 23.8 | ~8,126 |
| 全局 rules（~/.claude/rules/ 16文件） | 15,644 | 15.3 | ~5,215 |
| **规则重复开销**（16文件完全重叠） | 15,644 | 15.3 | ~5,215 |
| **静态注入小计** | **61,154** | **59.7** | **~20,385** |

**关键发现**：全局 rules 与项目 rules 有 **16 个完全相同的文件**（common/ 全部 8 个 + python/ 全部 5 个 + no-workaround.md + result-package.md + testing-override.md）。这 15,644 字节被**重复加载两次**，每次会话浪费 ~5,200 token。

### 1.2 Agent 定义

| 统计项 | 值 |
|--------|-----|
| Agent 数量 | 19 个 |
| 总字节 | 80,295 bytes (78.4 KB) |
| 平均每个 | 4,226 bytes (4.1 KB) |
| 最大 | gemini-challenger.md (6,879), meta-observer.md (6,833) |

Agent 定义在 spawn 时加载——单个 agent spawn = agent prompt + CLAUDE.md + 全部 rules = ~65 KB (~21,800 token)。

### 1.3 Skill 定义

| 统计项 | 值 |
|--------|-----|
| Skill 文件数量 | 18 个 |
| 总字节 | **139,417 bytes (136.1 KB)** |
| 最大单项 | methodology-v3.3.md: 33,732 bytes (33 KB) |
| 第二大 | methodology-v2.md: 30,430 bytes (29.7 KB) |

**关键发现**：meta-orchestration skill 连同 references/ 子目录总计 **79,138 bytes (77.3 KB, ~26,400 token)**。其中 methodology-v2.md 和 methodology-v3.3.md 两个文件就占 **64,162 bytes**——这两个文件几乎肯定是同一文档的不同版本，v2 存在的理由不明。

### 1.4 Hook 系统

| 统计项 | 值 |
|--------|-----|
| Hook 脚本数量 | 21 个 |
| 总字节 | 115,315 bytes (112.6 KB) |
| 总 hook 触发点 | 32 个 |

**每工具触发频次**：

| 工具 | PreToolUse | PostToolUse | 总计 |
|------|:----------:|:-----------:|:----:|
| Write | 4 | 6 | **10** |
| Edit | 4 | 6 | **10** |
| Bash | 1 | 4 | **5** |
| Task | 1 | 0 | 1 |
| TaskUpdate | 0 | 1 | 1 |
| Read | 0 | 0 | 0 |
| Glob | 0 | 0 | 0 |
| Grep | 0 | 0 | 0 |

**关键发现**：Hook 本身不直接消耗 context token（脚本代码不注入）。但 hook 的**输出**（systemMessage 或 block reason）会注入。大多数 hook 在 pass 时静默退出（exit 0，无输出）。但以下 hook 在触发时输出较大的 block reason：
- `topology-guard.sh`：git commit 后运行 BFS + manifest 检查，输出可含 20+ 行孤岛列表
- `genealogy-write-guard.sh`：316 行脚本，含语义检查，触发时输出多段警告
- `ceremony-completion-guard.sh`：306 行，Stop 时触发

Bash PostToolUse 有 4 个 hook（flow-continuity-guard + crystallization-guard + topology-guard），但它们都只在 `git commit` 成功时才实际执行逻辑，其他 Bash 命令直接 exit 0。**因此 hook 不是 context 膨胀的主因**。

### 1.5 数据文件（按需加载）

| 文件 | 字节 | KB | 估计 token |
|------|-----:|---:|----------:|
| pattern-buffer.yaml | 289,333 | 282.6 | **~96,444** |
| dispatch-dag.yaml | 38,650 | 37.7 | ~12,883 |
| 缠论知识库.md | 24,181 | 23.6 | ~8,060 |
| genealogy settled（152文件合计） | 752,059 | 734.4 | ~250,686 |
| sessions（97文件合计） | 176,013 | 171.9 | ~58,671 |

**关键发现**：
- `pattern-buffer.yaml`：**289 KB，单文件就占 ~96,000 token**——接近 200K context 的一半
- genealogy 152 个文件总计 752 KB——全量读取不可能，但审计类工位可能需要读取大量谱系文件
- sessions 97 个文件 172 KB——ceremony_scan.py 输出仅 1,489 bytes（正常），但如果有代码直接读取 session 文件则危险

### 1.6 Ceremony 扫描

| 统计项 | 值 |
|--------|-----|
| ceremony_scan.py 大小 | 17,742 bytes |
| 输出 JSON 大小 | **1,489 bytes** |

ceremony 扫描输出很小，不是问题。

---

## 2. 根因分析

### 根因 #1：规则重复加载（确定性浪费）——~5,200 token/会话

全局 `~/.claude/rules/` 和项目 `.claude/rules/` 有 16 个完全相同的文件。Claude Code 平台加载两份，导致每个会话（包括每个 agent spawn）浪费 ~5,200 token。

**严重程度**：中。单独看不致命，但乘以多个 agent spawn 则显著。

### 根因 #2：pattern-buffer.yaml 巨大（致命）——~96,000 token

289 KB 的 YAML 文件，任何需要读取 pattern-buffer 的操作会一次性消耗 ~96,000 token，占 200K context 的 48%。

**严重程度**：致命。这是审计类工位 context 耗尽的最可能直接原因。

### 根因 #3：meta-orchestration skill 中 methodology 冗余——~21,000 token

methodology-v2.md (30 KB) 和 methodology-v3.3.md (34 KB) 同时存在。如果 skill 加载时两个都读取，则浪费 ~21,000 token（v2 应该被 v3.3 取代）。

**严重程度**：高。skill 按需加载，但 meta-orchestration 是最常用的 skill。

### 根因 #4：审计工位的读取模式（结构性问题）

审计类工位（如 audit-upper）需要：
1. 读取多个谱系文件（每个 3-20 KB）
2. 读取 dispatch-dag.yaml (39 KB)
3. 可能读取 pattern-buffer.yaml (289 KB)
4. 加载 meta-orchestration skill (79 KB)
5. 加上静态注入 (61 KB)

最坏情况估算：静态注入(61KB) + agent(4KB) + skill(79KB) + pattern-buffer(289KB) + dag(39KB) + 20个谱系文件(~100KB) = **572 KB ≈ 190,000 token**——仅静态数据就几乎填满 200K context，留给实际工作的空间不到 10K token。

**严重程度**：致命。这解释了为什么审计工位反复 context limit 耗尽。

### 根因 #5：session 文件持续积累

97 个 session 文件，172 KB。如果某个操作扫描或加载全部 session，会额外消耗 ~58,000 token。ceremony 扫描只输出 1.5 KB 摘要（安全），但直接 glob + read 全部 session 则危险。

**严重程度**：中。取决于访问模式。

---

## 3. 修复方案（按优先级排列）

### P0：pattern-buffer.yaml 分片（预计节省 ~80,000 token/审计会话）

**问题**：289 KB 单文件不可能在单次会话中完整处理。
**方案**：将 pattern-buffer.yaml 拆分为多个按主题分类的小文件（每个 < 30 KB），加一个 index.yaml 索引。agent 只加载需要的分片。
**预期效果**：审计工位从"必须读取 289 KB"变为"读取 3-5 KB 索引 + 20-30 KB 相关分片"，节省 ~80,000 token。

### P1：消除规则重复（预计节省 ~5,200 token/agent spawn）

**问题**：16 个文件在 global 和 project rules 中完全重复。
**方案**：删除 `~/.claude/rules/` 中与项目 `.claude/rules/` 完全相同的文件（项目版本优先）。只保留项目独有的 3 个文件（no-patch-mentality.md、no-unnecessary-escalation.md、post-commit-flow.md）和全局独有的（无）。
**实操**：删除 `~/.claude/rules/common/` 全部 8 文件 + `~/.claude/rules/python/` 全部 5 文件 + `~/.claude/rules/no-workaround.md` + `~/.claude/rules/result-package.md` + `~/.claude/rules/testing-override.md`。
**预期效果**：每个 agent spawn 节省 ~5,200 token。5 个 agent 的蜂群节省 ~26,000 token。

### P2：清理 methodology-v2.md（预计节省 ~10,000 token/skill 加载）

**问题**：methodology-v2.md (30 KB) 已被 v3.3 取代，但仍存在于 skill references 中。
**方案**：确认 v3.3 完全覆盖 v2 内容后，删除 v2 或将其移至 archive。
**预期效果**：meta-orchestration skill 加载节省 ~10,000 token。

### P3：session 文件归档（预计节省 ~50,000 token/误读场景）

**问题**：97 个 session 文件持续积累。
**方案**：保留最近 10 个 session，其余移至 `.chanlun/sessions/archive/`。ceremony 扫描已只读最新一个。
**预期效果**：降低误读全部 session 的风险。预防性措施。

### P4：审计工位架构改造（根本性修复）

**问题**：审计工位需要大量数据但 200K context 不够。
**方案**：
1. 审计工位采用**分阶段扫描**模式——不在单个 context 中读取所有文件，而是：
   - Phase 1：读取 index/摘要文件，确定需要深入的区域
   - Phase 2：只读取相关的具体文件
2. 为大型数据文件提供**摘要视图**（summary 版本 < 5 KB），审计工位先读摘要，按需展开
3. 考虑将 dispatch-dag.yaml (39 KB) 也拆分或提供摘要版

**预期效果**：审计工位从"填满 context 后失败"变为"在 context 预算内完成"。

---

## 4. 预期效果汇总

| 修复项 | 优先级 | 预计节省 (token) | 实施难度 |
|--------|:------:|:----------------:|:--------:|
| pattern-buffer 分片 | P0 | ~80,000/审计会话 | 中（需要定义分片策略） |
| 规则去重 | P1 | ~5,200/agent spawn | 低（直接删文件） |
| methodology-v2 清理 | P2 | ~10,000/skill 加载 | 低（删除或归档） |
| session 归档 | P3 | ~50,000（预防） | 低（移动文件） |
| 审计工位架构改造 | P4 | 根本性解决 | 高（需要重新设计 agent prompt） |

**P0+P1+P2 合计**：每个审计工位会话可节省 ~95,000 token，相当于释放 200K context 的 47.5%。

---

## 5. 附录：完整目录树尺寸

```
静态注入层（每会话必加载）:
  CLAUDE.md                           5,487 bytes
  .claude/rules/ (19 files)          24,379 bytes
  ~/.claude/rules/ (16 files)        15,644 bytes
  重复开销                            15,644 bytes
  ─────────────────────────────────────────────
  小计                                61,154 bytes (~20K tokens)

Agent 层（spawn 时加载）:
  .claude/agents/ (19 files)         80,295 bytes (~27K tokens)
  每个 agent 平均                      4,226 bytes

Skill 层（按需加载）:
  .claude/skills/ (18 files)        139,417 bytes (~46K tokens)
  其中 meta-orchestration alone       79,138 bytes (~26K tokens)
  其中 methodology-v2+v3.3            64,162 bytes (~21K tokens)

数据层（按需加载）:
  pattern-buffer.yaml               289,333 bytes (~96K tokens)  ← 头号杀手
  dispatch-dag.yaml                  38,650 bytes (~13K tokens)
  genealogy/ (152 files)            752,059 bytes (~251K tokens)
  sessions/ (97 files)              176,013 bytes (~59K tokens)
  缠论知识库.md                       24,181 bytes (~8K tokens)

Hook 层（不直接消耗 context）:
  .claude/hooks/ (21 scripts)       115,315 bytes
  触发点总数                              32 个
  高频工具（Write/Edit）每次              10 个 hook
  但仅输出注入 context，pass 时静默
```
