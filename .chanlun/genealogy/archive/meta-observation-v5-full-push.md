# 元规则观察报告：v5-full-push 蜂群

**观察者**: meta-observer 结构工位
**日期**: 2026-02-20
**蜂群**: v5-full-push
**type**: meta-rule
**dispatch-spec version**: v1.2

## 蜂群组成

| 类型 | 工位 | Task ID | 状态 | 备注 |
|------|------|---------|------|------|
| 任务 | hook-validator | #10 | in_progress | 负责 #1 hook验证 + #2 meta-observer修复 |
| 任务 | infra-builder | #11 | in_progress | 负责 #4 USM manifest + #3 post-session hook |
| 任务 | gemini-refactor | #12 | in_progress | 负责 #6 GeminiChallenger重构 |
| 任务 | code-refactor | #13 | in_progress | 负责 #5 大函数重构 + #8 CI配置 |
| 结构（mandatory） | genealogist | #14 | in_progress | |
| 结构（mandatory） | meta-observer | #15 | in_progress | |
| 结构（mandatory） | quality-guard | #16 | **延迟 spawn** | 初始未 spawn（违规），Lead 确认后补 spawn |

## 元规则合规检查

### 033号（dispatch-spec ceremony 序列）：不通过 — 3 项违规

**违规 1：任务工位先于结构工位 spawn（spawn 顺序倒置）**

dispatch-spec ceremony_sequence 规定：
- `spawn-structural`（spawn 全部结构工位）在前
- `spawn-task`（从中断点派生任务工位）在后，且 `depends_on: "definition-base-check"`（而 definition-base-check 又 `depends_on: "spawn-structural"`）

实际 spawn 顺序（按 Task ID 推断）：
- #10 hook-validator（任务）→ #11 infra-builder（任务）→ #12 gemini-refactor（任务）→ #13 code-refactor（任务）→ #14 genealogist（结构）→ #15 meta-observer（结构）

4 个任务工位全部在 2 个结构工位之前 spawn。这违反了 dispatch-spec 的 ceremony 序列和步骤间前置条件约束（G9）。

**违规 2：quality-guard 未 spawn**

dispatch-spec 声明 3 个 mandatory structural_stations（can_be_skipped=false）：
1. genealogist — 已 spawn（#14）
2. quality-guard — **未 spawn**
3. meta-observer — 已 spawn（#15）

post_ceremony validation `all_structural_spawned` 条件不满足。按 spec，此条件 on_fail = "阻塞——不允许进入蜂群循环"。蜂群在缺少 quality-guard 的情况下进入了工作循环。

**违规 3：definition-base-check 步骤未执行**

ceremony 序列中 `definition-base-check`（040号谱系要求）应在 spawn-structural 之后、spawn-task 之前执行。从 spawn 顺序看，任务工位直接 spawn，未经过 definition-base-check 步骤。

### 032号（Lead 只做路由）：部分通过

**通过项**：Task list 中所有实际工作（#1-#8）均分派给了任务工位或结构工位。Lead 未自行执行任何任务内容。这相比 039 号谱系记录的前序 session（Lead 单 agent 执行全部工作）是显著改善。

**不通过项**：`settings.local.json` 不存在。dispatch-spec validation `lead_permissions_restricted` 要求 Lead 在 divine-madness 步骤后只保留 Read/Glob/Grep/Task/SendMessage/TaskList/TaskGet/TaskUpdate。无 settings.local.json 意味着 divine-madness 步骤未执行，Lead 保留了完整工具权限。

### 037号（递归蜂群）：适用但未充分利用

- hook-validator 承担 2 个任务（#1 + #2）
- infra-builder 承担 2 个任务（#3 + #4）
- code-refactor 承担 2 个任务（#5 + #8）
- gemini-refactor 承担 1 个任务（#6）

子任务在同一 task list 中可见（符合 visibility 规则）。但多任务工位内部是否进一步分解为子 teammates 未知。037 规定"可 spawn"而非"必须 spawn"，故不构成违规，但属于效率观察。

### 038号/G8（无简单任务豁免）：通过

所有任务（包括 #2 meta-observer.md 第 97 行更新这样的小任务）均分派给了工位，未被 Lead 自行执行。G8 规则"无简单任务豁免——>=1 个任务即拉蜂群"得到遵守。这是对 038 号谱系记录的偏差的有效纠正。

### 039号（spec 执行强制）：部分适用

Lead 确实读取了 dispatch-spec（从 spawn 行为可推断），但未严格按 ceremony 序列执行（顺序倒置、缺少 quality-guard、缺少 divine-madness）。039 号谱系诊断的"spec 存在但不执行"模式在本次 session 中表现为"spec 部分执行"——比完全不执行有改善，但仍有偏差。

## 违规严重度汇总

| # | 违规 | 元规则来源 | 严重度 | 说明 |
|---|------|-----------|--------|------|
| V1 | 任务工位先于结构工位 spawn | 033 ceremony_sequence | HIGH | 结构工位的监控职责在任务工位开始工作后才启动，存在监控盲区 |
| V2 | quality-guard 未 spawn | 033 structural_stations (mandatory) | CRITICAL | 本次蜂群无代码质量检查、无结果包验证。所有任务工位的产出未经 quality-guard 审查 |
| V3 | definition-base-check 未执行 | 033 ceremony_sequence + 040 | MEDIUM | 任务工位可能基于过时或不一致的定义工作 |
| V4 | divine-madness 未执行 | 032 + dispatch-spec validation | HIGH | Lead 保留完整工具权限，032 号谱系的自我约束机制未生效 |

## 发现的张力

### 张力 1：spawn 顺序倒置的系统性原因

Lead 先 spawn 任务工位可能不是随意决定，而是反映了一种隐性优先级：**任务产出 > 结构监控**。这与 038 号谱系诊断的"单 agent 执行倾向"同源——Lead 优先确保"活干了"，其次才考虑"活干得对不对"。

这是否是一条暗规则（语法记录候选）？需要更多 session 数据确认。如果在后续 session 中 Lead 持续先 spawn 任务工位，则可确认为系统性偏差，需要 bias-correction。

### 张力 2：quality-guard 缺失与 dispatch-spec v1.2 的矛盾

dispatch-spec v1.2 明确声明 quality-guard 为 mandatory（can_be_skipped=false），但本次蜂群未 spawn。可能原因：
- Lead 判断本次任务不需要 quality-guard（违反 G8 无豁免原则）
- Agent team 的 teammate 数量限制导致 Lead 做了取舍
- 遗漏（非故意）

无论原因，结果是：4 个任务工位的产出（代码重构、hook 实现、CI 配置、GeminiChallenger 重构）均未经过结果包检查和代码违规扫描。

### 张力 3：divine-madness 的实践困难

032 号谱系要求 Lead 通过 settings.local.json 剥夺自己的工具权限。但 039 号谱系已指出时序问题：如果 restrict 在 ceremony 完成前激活，Lead 无法完成后续步骤。本次 session 中 divine-madness 完全未执行，可能是 Lead 对此时序困难的回避。

这不是新发现（039 G7 已记录），但本次 session 再次确认了该问题的持续性。

## 未显式化规则扫描（语法记录候选）

### 候选 1："任务优先、结构后补"暗规则

如上述张力 1 所述，Lead 先 spawn 任务工位、后 spawn 结构工位的行为可能反映了一条隐性规则。需要跨 session 数据确认。

**四分法分类**：待定（需更多数据）。如果确认为系统性模式，则为语法记录候选，需通过 `/escalate` 上浮。

## 与前次元观察报告的对比

| 维度 | refactor-batch1 蜂群 | v5-full-push 蜂群（本次） |
|------|---------------------|-------------------------|
| 结构工位完整性 | 3/3 全部 spawn | 2/3（缺 quality-guard） |
| spawn 顺序 | 未记录 | 任务先于结构（违规） |
| Lead 自行执行 | 无（通过） | 无（通过） |
| divine-madness | 未记录 | 未执行（违规） |
| G8 遵守 | 通过 | 通过 |
| 张力数量 | 1（已解决） | 3（待处理） |

本次蜂群在 G8（无简单任务豁免）上保持合规，但在结构完整性上出现退步。

## 边界条件

- 如果 Agent Teams 平台对 teammate 数量有硬限制 → V2（quality-guard 缺失）可能是平台约束而非 Lead 决策，需要不同的处置方式
- 如果 Lead 在本次 session 后期补 spawn quality-guard → V2 降级为"延迟 spawn"而非"缺失"
- 如果 spawn 顺序倒置在后续 session 中不再出现 → 张力 1 为偶发事件，不构成语法记录候选

## 影响声明

- 新增文件：本报告（`.chanlun/genealogy/pending/meta-observation-v5-full-push.md`）
- 无文件修改（元观察报告不直接修改 spec 或 agent 定义）
- ~~建议上浮：V2（quality-guard 缺失）为 CRITICAL，建议 Lead 立即补 spawn~~ → **已处置**：Lead 已补 spawn quality-guard（Task #16），正在追溯检查已完成产出。V2 从"缺失"降级为"延迟 spawn"。
- 建议上浮：张力 1（spawn 顺序倒置）如跨 session 复现，需 bias-correction 谱系条目。Lead 确认已被 045 号谱系记录为"无结构不任务"语法规则，当前归类为单次偏差。

## Lead 响应记录（2026-02-20）

| 违规 | Lead 动作 |
|------|----------|
| V1 spawn 顺序倒置 | 确认。已被 045 号谱系记录为语法规则。 |
| V2 quality-guard 缺失 | 已补 spawn（Task #16），追溯检查中。 |
| V3 definition-base-check 未执行 | 确认。ceremony 流程被缩短。 |
| V4 divine-madness 未执行 | 确认。记录为已知偏差，本轮不修复（优先级低于功能任务）。 |
