# autocompact 恢复 — 诊断 + 修复报告（task #40）

- **工位**：autocompact-restore（蜂群基础设施工位）
- **日期**：2026-06-23
- **主仓库绝对路径**：`/Users/silencehan/Projects/NewChanlun`
- **改动文件**：`.claude/hooks/ceremony-completion-guard.sh`（Stop-Guard，新增"检查 0：context 临界放行"）
- **未改动**：`.claude/settings.json`（autocompact 配置已正确，详见下文）、`.claude/hooks/precompact-save.sh`（状态保存链完整）

---

## 一、现象与诊断

**现象**：context 临界（75%+）但 autocompact 未自动拉起。

**核心假说（编排者/Lead 给定）确认成立**：Stop-Guard（`ceremony-completion-guard.sh`）在蜂群任务活跃时每轮 `decision: block` 阻止停机 → autocompact 所需的 **turn 边界（停机点）永不到达** → autocompact 永不 fire → context 突破 75% 阈值后无界增长。

### L2 实测证据（本 session 实时数据）

| 度量 | 值 | 来源 |
|------|-----|------|
| context 窗口 | **1,000,000 tokens**（Opus 4.8 [1m]） | 多 session transcript peak 994k–1,010,080；sonnet session 封顶 ~141k 印证 200k 窗口 |
| autocompact 阈值 | 75% = **750,000 tokens** | `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE=75` + 谱系 407 |
| **Lead session 14c95478 峰值 context** | **779,143 tokens = 77.9%** | transcript usage（input+cache_creation+cache_read+output） |
| Lead session 当前 context | 148,313 = 14%（峰值后已 compact） | 同上 |

Lead session 突破 750k 阈值达到 779k 才回落——**autocompact 被延迟到 75% 阈值之上**，正是报告的现象。回落说明最终仍 compact 了（推测靠 145号智能熔断恰好连续 3 次停滞触发，或人工 /compact），但**非确定性**——依赖熔断恰好命中。

### 机制根因（计数模式，沿 407/550 先例）

- autocompact 在 turn 边界触发，不是 skill 监听的事件（**谱系 550 已结算此点**）。
- Stop-Guard 检查 1–5 在蜂群任务活跃（`ACTIVE_TASKS > 0`）时每轮 block，且任务状态持续变化使 145号智能熔断（连续 3 次状态不变才放行）很少命中 → turn 边界几乎永不到达。
- Stop-Guard **从无任何 context/autocompact 感知**（`git log` + grep 确认：hook 历史中 `autocompact|context|compact` 命中数 = 0）→ 这是**能力缺口，非回归**。

---

## 二、修复（严格解，no-workaround）

### 真优先级矛盾的严格解

| 矛盾两端 | 取舍 |
|---------|------|
| Stop-Guard 蜂群持续性（活跃任务时不停机） | context 临界时**让位** |
| context 临界 compact 必要性（避免溢出/状态崩坏） | **优先** |

**严格解依据**：蜂群状态已持久化在文件系统（`~/.claude/tasks/`、`teams/`、`.chanlun/genealogy/`、`.chanlun/sessions/`），compact 后由 `session-start-ceremony.sh` 重注入角色锚点恢复（137号/407号机制）。因此"context 临界时 compact 优先于蜂群持续"不丢失信息——这不是妥协方案，是矛盾的严格存在论表达：**蜂群的持续性载体是文件系统，不是单个 session 的 context**。

### 实现

在 Stop-Guard 顶部（COUNTER 定义后、所有 block 检查之前）新增**检查 0**：

1. 从 Stop hook 输入的 `transcript_path` 解析最近一条非 sidechain assistant 消息的 `usage`，计算当前 context = `input_tokens + cache_creation_input_tokens + cache_read_input_tokens + output_tokens`。
2. 窗口检测：env `CLAUDE_CTX_WINDOW_TOKENS` 覆盖 > 模型族回退（含 "sonnet" → 200k，否则 1M）。默认不写死、可配置（遵守 coding-style "no hardcoded values"）。
3. `CTX_PCT ≥ 放行阈值`（env `CLAUDE_STOPGUARD_RELEASE_PCT`，默认 **85**）时：
   - 调 `write_session.sh` 落盘（precompact-save.sh 会在 PreCompact 再存一次 = 双保险）；
   - 重置熔断计数器；
   - 输出 `{continue: true, systemMessage: ...}`（**无 `decision: block`**）→ `exit 0` → **放行停机** → autocompact 取回 turn 边界 → fire。
4. `CTX_PCT < 85%` 或无 transcript → 完全透明落穿到检查 1–5（蜂群持续性不变）。

### 阈值关系

- 放行阈值 **85% > autocompact 阈值 75%**：75%–85% 区间给蜂群 ~100k tokens 运行余量后强制放行；85%（850k）留 150k headroom 避免 1M 硬上限溢出。
- 放行后 context 仍 ≥75%，autocompact 在 turn 边界确定性 fire。

### fail-closed 安全性

transcript 读取异常 → `CTX_PCT=0` → 不放行 → 维持旧行为（蜂群持续）。**只在确信 context 临界时放行，绝不误弃蜂群**。

---

## 三、验证

- `bash -n` 语法检查：**PASS**。
- 功能测试（构造 transcript）：
  - 87% context → 输出 `continue:true` + systemMessage，**无 `decision:block`**，`exit 0`（放行）✓
  - 50% context → 落穿到正常检查，`exit 0`（正常）✓
- 状态保存链：settings.json `PreCompact → precompact-save.sh → write_session.sh` 完整未动；检查 0 放行路径额外预存一次 ✓

---

## 四、结果包六要素

1. **结论**：Stop-Guard 新增 context 临界放行（检查 0）。context ≥85%（1M 窗口，可配置）时无条件放行停机，让 autocompact 在 turn 边界 fire。修复"Stop-Guard 持续 block → autocompact 永不触发"的 livelock。settings.json 无需改动（autocompact 75% + PreCompact hook 已正确）。

2. **定义依据**：autocompact 在 turn 边界（停机点）触发（谱系 550："autocompact 不是 skill 监听的事件"）；Stop-Guard 检查 1–5 在 `ACTIVE_TASKS>0` 时每轮 block（hook 第 291–335 行）；145号智能熔断要求"连续 3 次任务态不变"，活跃蜂群任务态持续变化使其少命中 → turn 边界几乎永不到达。L2 实测 779k/1M=77.9% 越过 75% 阈值未及时 compact，满足"被挡住"的全部条件。

3. **边界条件**（结论翻转条件）：
   - 若 Claude Code 改为 turn 边界**之外**（如每次 API call 前）检查 autocompact → 本修复非必要（但无害，仍是溢出兜底）。
   - 若窗口非 1M 且未设 `CLAUDE_CTX_WINDOW_TOKENS`、模型名不含 "sonnet" → 默认按 1M 算，小窗口下百分比偏低可能不放行（需设 env 覆盖）。
   - 若放行阈值设过低（≈75%）→ 蜂群一到 75% 即被 compact 打断，运行余量归零（故默认 85% 而非 75%）。

4. **下游推论**：
   - autocompact 现在对蜂群 session 确定性触发 → 谱系 407 的"compact 后行为模式丢失"会**更频繁**地被触发，`session-start-ceremony.sh` 的行为锚点重注入质量成为关键路径（407 未闭合的缺口被本修复放大暴露）。
   - 谱系 550 的"两层免疫"判断需扩展：快层不仅在 compact 期**静默**，此前还因 Stop-Guard 而**被阻止进入 compact**——这是 550 未覆盖的新维度（见谱系引用）。

5. **谱系引用**：
   - **407**（lead-role-boundary-compact-regression，settled）：autocompact 基础设施审查 + 三层恢复不对等（状态指针保存、行为模式丢失）+ ceremony 重注入。本修复的直接上游，确认窗口=1M/阈值=75%/750k。
   - **550**（genealogy-immune-two-layer-compact-silence，settled）：否定"事件驱动 = 谱系免疫完整"，因"autocompact 不是 skill 监听的事件"。本修复揭示**新维度**——autocompact 不仅未被监听，还曾被 Stop-Guard **主动阻止 fire**。建议 genealogist 评估是否需新谱系记录（语法记录类：Stop-Guard ⊥ autocompact 的优先级裁决）。
   - **145**（Stop-Guard 智能熔断）：本修复与熔断并存（检查 0 在熔断之前，是确定性放行；熔断是停滞兜底）。
   - **137**（否定性禁令对行为层无效 / 文件系统注入恢复）：严格解依据——蜂群持续性载体是文件系统。
   - **048/044/069**（Stop-Guard 谱系链）：检查 0 是该链的 context 维扩展。

6. **影响声明**：改动 `.claude/hooks/ceremony-completion-guard.sh`（+约 50 行，纯新增，无删改既有逻辑）。影响 Stop 事件的放行决策（仅在 context ≥85% 时新增放行路径）。不影响检查 1–5 在非临界时的行为。不改 settings.json / permission / security。新增两个可选 env 变量（`CLAUDE_STOPGUARD_RELEASE_PCT` 默认 85、`CLAUDE_CTX_WINDOW_TOKENS` 默认按模型族）。

---

## 五、交付

改动已 `bash -n` 验证 + 功能测试通过。**未自行 commit**（settings.json 敏感约束 + Lead 统一 commit 流程）——交 Lead commit `.claude/hooks/ceremony-completion-guard.sh`。
