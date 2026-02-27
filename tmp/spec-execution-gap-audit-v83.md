# 声明-能力缺口审计 v83

**审计时间**: 2026-02-27
**审计范围**: "RTAS蜂群默认持久化向下无限递归蜂群"声明 + 系统全局声明-能力一致性
**方法论**: spec-execution-gap skill (036号谱系) — 逐层拆解声明，交叉验证实现

---

## 一、核心声明拆解："RTAS蜂群默认持久化向下无限递归蜂群"

将此声明拆解为五个独立子声明，逐个审计：

| # | 子声明 | 含义 | 审计结论 | 分类 |
|---|--------|------|----------|------|
| 1 | **RTAS**（递归拓扑异步自指） | 蜂群具备四项特征：递归/拓扑/异步自指/结晶 | 部分缺口 | 见 §1.1 |
| 2 | **默认** | 蜂群是默认工作模式，不是可选优化 | 部分缺口 | 见 §1.2 |
| 3 | **持久化** | 每个退出路径都持久化状态 | 部分缺口 | 见 §1.3 |
| 4 | **向下** | 子蜂群同样是完整蜂群 | 部分缺口 | 见 §1.4 |
| 5 | **无限递归** | 递归深度无人为限制 | 真缺口 | 见 §1.5 |

---

### 1.1 RTAS（递归拓扑异步自指）— 部分缺口

**声明来源**: CLAUDE.md 原则15、069号谱系、swarm-architecture skill

**四项特征审计**:

| 特征 | 声明 | 实现状态 | 缺口 |
|------|------|----------|------|
| **递归** | teammate 通过 TeamCreate 创建子 team | Claude Code Agent Teams 平台支持，sub-swarm-ceremony skill 有完整流程 | 实现存在（见1.4/1.5细节缺口） |
| **拓扑** | DAG 格式的 dispatch-dag.yaml | dispatch-dag.yaml 定义了完整 DAG，但 ceremony_scan.py **未实现 DAG 解析器**（脚本自述："本脚本并未实现 DAG 解析器——扫描顺序由代码逻辑决定"） | **部分缺口**：DAG 是声明层文档，不是代码执行的拓扑约束 |
| **异步自指** | t 时刻审查 t-1 产出 | async_self_reference.py 存在，集成在 ceremony_scan.py 183号目B | 实现存在 |
| **结晶** | 产出写入文件 | skill-crystallizer 在 event_skill_map 中声明，pattern-buffer 机制存在 | 实现存在 |

**关键缺口**: ceremony_scan.py 第17-19行自述："dispatch-dag.yaml 的 ceremony_sequence 定义了 DAG 格式的 nodes+depends_on，但本脚本并未实现 DAG 解析器——扫描顺序由代码逻辑决定（roadmap -> session -> fallback）。DAG 的 ceremony_sequence 由 LLM 解释执行"。

这意味着：**拓扑（T）在声明层是 DAG，在执行层是 LLM 解释的线性序列**。DAG 依赖关系（depends_on）没有被代码强制，全靠 LLM 自觉遵守。按 spec-execution-gap 方法论（036号）：声明-能力不一致。

---

### 1.2 默认 — 部分缺口

**声明来源**: CLAUDE.md 原则10（"蜂群是默认工作模式"）、原则15（"真递归是默认架构模式"）

**实现审计**:

- `ceremony_scan.py` 确实在扫描后输出 workstations 列表，Lead 据此 spawn 工位
- dispatch-dag.yaml task_template rules 声明"无简单任务豁免——>=1 个任务即拉蜂群"
- 但实际是否拉蜂群完全取决于 **LLM 对 ceremony_scan 输出的解释和执行意愿**
- **没有 hook 或代码强制** "必须 spawn 至少 2 个 teammate"

**缺口分类**: 部分缺口。规则在 CLAUDE.md 和 dispatch-dag.yaml 中声明，但执行依赖 LLM 自觉。按 016号发现：规则没有代码强制就不会被执行（知道规则 != 执行规则）。

不过，agent-team-enforce.sh hook 确实在 PreToolUse:Task 上执行，这提供了部分强制。但它检查的是 team_name 参数是否存在，不是"是否应该拉蜂群"。

---

### 1.3 持久化 — 部分缺口

**声明来源**: 五约束 1a（物理持久化）、swarm-architecture skill E节（热启动机制）

**退出路径审计**:

| 退出路径 | 持久化机制 | 覆盖范围 | 缺口 |
|----------|-----------|----------|------|
| **正常 compact** | precompact-save.sh hook | .chanlun/sessions/ 写入 session 文件 | 覆盖完整 |
| **会话关闭** | 无自动机制 | 依赖最近的 precompact 或手动 session | **真缺口**：如果 compact 从未触发就关闭，最新状态丢失 |
| **ceremony 原子链** | ceremony_push_and_rescan.sh | git add 已知文件类型，git push | 见下方详细分析 |
| **工位完成** | 工位通过 SendMessage 回传结果 | 结果存在于 Lead 上下文中，不一定持久化到文件 | **部分缺口**：工位产出如果 Lead 未 commit 就 compact，可能丢失 |

**ceremony_push_and_rescan.sh 的 git add 范围分析**:

```bash
# 第25-26行
git add -- '*.py' '*.md' '*.yaml' '*.sh' '*.json' '*.jsonl' 2>/dev/null
git reset HEAD -- tmp/ 2>/dev/null
```

- **覆盖**: .py .md .yaml .sh .json .jsonl 扩展名的文件
- **排除**: tmp/ 目录被显式 reset
- **缺口**: `tmp/` 下的文件不被 ceremony 自动持久化。但从 git status 看，tmp/ 下有大量未跟踪文件（约 200+ 个 .py .md 文件）。这些文件是审计报告、验证脚本、诊断结果等。

**tmp/ 排除是否是缺口？** 需要区分：
- tmp/ 下的文件本身设计为临时文件（不需要持久化到 git）→ 不是缺口
- 但如果 tmp/ 下的文件包含**不可再生的分析结论**（如 Gemini 验证结果），且没有被结晶到 .chanlun/ 下 → **部分缺口**：结论可能随文件删除而丢失

**结论**: 持久化机制存在但有盲区。核心 .chanlun/ 目录被完整持久化，但：
1. 非 compact 触发的会话关闭没有自动持久化
2. tmp/ 下的临时分析结论不被 ceremony 持久化（设计意图 vs 实际风险的张力）

---

### 1.4 向下（子蜂群完整性）— 部分缺口

**声明来源**: 原则15（"子蜂群同样是递归拓扑异步自指蜂群——子 team 不是简化版 agent pool，它复制父蜂群的完整结构"）、097号五特征 DAG 模板

**实现审计**:

sub-swarm-ceremony skill 定义了完整的子蜂群创建流程，包括四类节点（任务/审查/结晶/异质审计）。

| 五特征 | 声明 | 子蜂群实现 | 缺口 |
|--------|------|-----------|------|
| **拓扑** | >=2 并行任务节点 | sub-swarm-ceremony 步骤2要求创建四类节点 | 依赖 LLM 遵守，无代码强制 |
| **异步自指** | 审查节点 | 步骤4要求 Lead 执行审查 | 依赖 LLM 遵守 |
| **结晶** | 产出写入文件 | 步骤5要求写入 session | 依赖 LLM 遵守 |
| **状态管理** | 独立 TaskList | TeamCreate 创建独立 team | **实现存在**（平台支持） |
| **异质验证** | 异质审计节点 | 步骤4要求调用 Gemini challenger | **部分缺口**：无 hook 强制子蜂群必须有异质审计 |

**关键缺口**: 子蜂群的五特征全部依赖 LLM 读取 sub-swarm-ceremony skill 后自觉遵守。没有任何 hook 在 TeamCreate 时验证子蜂群是否具备五特征。按 016号/032号：声明 != 能力。

---

### 1.5 无限递归 — 真缺口

**声明来源**: 原则15（"递归深度无人为限制，由 topology-manager 的收敛信号自然终止"）

**实现审计**:

1. **声明**: "递归深度无人为硬限制"（dispatch-dag.yaml recursion_rules 第4条）
2. **同一文件的矛盾声明**: swarm-architecture skill 明确说"递归深度工程上限 ≈ 3-4 层"
3. **实际约束**:
   - Claude Code Agent Teams 并发 agent 数有平台限制
   - 每层递归消耗 API token（成本线性增长）
   - 每层递归引入视差 Gap 累积
   - sub-swarm-ceremony skill 第139行："如果已经是 L3，优先在当前层扁平执行而非继续递归"

**声明-实现矛盾**:
- 声明1：无人为限制
- 声明2（同一系统）：工程上限 3-4 层
- 实现：sub-swarm-ceremony 建议 L3 停止递归

这三者构成内部矛盾。"无限递归"声明与"3-4 层上限"声明不一致。

**进一步分析**: 系统区分了"铁壁垒终止"和"OOM 终止"（163号/164号谱系），声明的意图是递归应到达铁壁垒而非被铁笼子截断。但实际的 L3 建议是一种**软人为限制**。

**缺口分类**: 真缺口。声明"无限递归"但实际有 3-4 层软限制。声明应修改为"有限深度递归（工程上限 3-4 层，由资源约束和视差 Gap 累积自然终止）"。

---

## 二、全局声明-能力缺口审计

### 2.1 四分法过滤 — 部分缺口

**声明**: no-unnecessary-escalation.md 要求所有 agent 在提问前执行四分法分类

**实现**:
- ceremony-completion-guard.sh（Stop hook）检查5检测四分法违规模式（确认请求/选择上浮）
- 但该检查基于**正则匹配**（`待确认|以上理解是否正确|...`），只能检测表面模式
- 无法检测语义层面的四分法违规（如用陈述句包装的确认请求）
- Hook 不区分 Lead 和工位（228号已确认平台限制）

**缺口分类**: 部分缺口。正则匹配覆盖了常见违规模式，但无法覆盖全部。

### 2.2 并行默认 — 部分缺口

**声明**: lead-parallel-dispatch.md 要求 Lead 并行执行独立操作

**实现**:
- 规则在 .claude/rules/ 中声明（Claude Code 强制加载）
- 但 LLM 是否真的并行调用取决于模型行为，无代码强制
- 没有 hook 检测"Lead 是否将可并行操作串行化"

**缺口分类**: 部分缺口。规则通过 rules/ 被注入到 LLM prompt，但执行依赖模型自觉。

### 2.3 不动点终止 — 部分缺口

**声明**: ceremony_sequence.terminate_condition 定义了不动点条件（workstations 为空或与上轮相同）

**实现**:
- ceremony_scan.py 输出 `clean_terminate` 字段
- flow-continuity-guard.sh 在 ceremony_scan.py 执行后注入指令要求检查不动点
- **已知问题**: 下游推论重复检出。downstream_audit.py 报告 3 个 unresolved，execution_rate 87%。如果 unresolved 项实际已解决但因检测逻辑不完善被报为 unresolved → ceremony_scan 会为它们生成工位 → **永远无法达到不动点**

**验证**: 当前 downstream_audit --summary 输出 3 个 unresolved。需要确认这 3 个是真 unresolved 还是假阳性。verification_hints 机制（154号-2）旨在减少假阳性，但可能未完全覆盖。

**缺口分类**: 部分缺口。不动点检测逻辑存在，但假阳性可能阻止系统到达不动点。

### 2.4 ceremony_scan.py 的 downstream_audit — 伪缺口（需确认）

**声明**: resolved 的下游推论不应被报为 unresolved

**实现**:
- downstream_audit.py 有多层检测：inline 标记、negation_map、交叉引用、verification_hints、manual overrides
- 当前 3 个 unresolved 可能是真 unresolved 或假阳性
- 154号-2 优化和 overrides 机制提供了假阳性修正路径

**缺口分类**: 需进一步确认。如果 3 个 unresolved 是真 unresolved → 伪缺口（系统正确报告）。如果是假阳性 → 部分缺口（检测逻辑不完善）。

### 2.5 DAG 拓扑排序 vs 线性扫描 — 真缺口

**声明**: dispatch-dag.yaml ceremony_sequence 使用 DAG 格式（nodes + depends_on）

**实现**: ceremony_scan.py 第17-19行明确自述未实现 DAG 解析器。扫描顺序是硬编码的 roadmap -> session -> fallback。

**影响**: ceremony_sequence 中的 depends_on 约束没有被代码执行。例如 "derive-work" depends_on ["scan-definitions", "scan-genealogy"]，但代码中没有检查这些依赖是否满足就直接执行。

**缺口分类**: 真缺口。DAG 声明存在但解析器未实现。

---

## 三、缺口汇总

### 真缺口（声明了但实现完全缺失或内部矛盾）

| # | 缺口 | 声明 | 实际 | 修复方案 |
|---|------|------|------|----------|
| G1 | 无限递归声明矛盾 | "递归深度无人为限制" | 同系统声明"3-4层上限"，sub-swarm-ceremony 建议 L3 停止 | 修改声明为"有限深度递归（资源约束自然终止，工程上限约 3-4 层）" |
| G2 | DAG 解析器缺失 | ceremony_sequence 是 DAG 格式 | ceremony_scan.py 未实现 DAG 解析器 | 选项A：实现 DAG 拓扑排序；选项B：修改声明为"优先级线性扫描" |

### 部分缺口（有实现但覆盖不完整）

| # | 缺口 | 声明 | 实际覆盖 | 缺失覆盖 |
|---|------|------|----------|----------|
| P1 | 子蜂群五特征强制 | 子蜂群必须具备五特征 | sub-swarm-ceremony skill 定义流程 | 无 hook 在 TeamCreate 时验证五特征 |
| P2 | 会话关闭持久化 | 每个退出路径持久化 | compact 有 precompact-save.sh | 非 compact 会话关闭无自动持久化 |
| P3 | 蜂群默认模式强制 | >=1 任务即拉蜂群 | 规则在 rules/ 中声明 | 无 hook 强制 spawn 行为 |
| P4 | 四分法过滤 | 所有 agent 提问前四分法 | Stop hook 正则检测 | 语义层违规不可检测 |
| P5 | 并行默认强制 | Lead 并行执行独立操作 | 规则在 rules/ 中声明 | 无检测 Lead 串行化行为的机制 |
| P6 | 不动点终止 | 不动点检测 | clean_terminate 字段存在 | downstream_audit 假阳性可能阻止不动点 |
| P7 | tmp/ 分析结论持久化 | 产出持久化 | .chanlun/ 完整持久化 | tmp/ 下不可再生的分析结论未自动结晶 |

### 伪缺口（声明与实现一致）

| # | 项目 | 说明 |
|---|------|------|
| F1 | 热启动机制 | L0/L1/L2 三级恢复均有实现（precompact-save.sh + session-start-ceremony.sh + ceremony_scan warm_start） |
| F2 | 异步自指 | async_self_reference.py 存在且集成在 ceremony_scan.py |
| F3 | ceremony 原子链 | ceremony_push_and_rescan.sh 实现了步骤7-10原子链；ceremony-step-guard.sh + flow-continuity-guard.sh 提供运行时强制 |
| F4 | 编排者代理 | Gemini decide 模式在 orchestrator_proxy 声明，gemini-challenger agent 存在 |
| F5 | ceremony 状态管理 | ceremony_state.py 提供 write_step/read_step/clear_step/is_in_ceremony |

---

## 四、总体评估

**声明 "RTAS蜂群默认持久化向下无限递归蜂群" 的逐组件评估**:

| 组件 | 评级 | 说明 |
|------|------|------|
| R（递归） | 基本具备 | 平台支持 + skill 定义，缺代码强制 |
| T（拓扑） | 声明层存在、执行层缺失 | DAG 定义完整但解析器未实现 |
| A（异步自指） | 具备 | async_self_reference.py 存在 |
| S（结晶/自指） | 基本具备 | pattern-buffer + skill-crystallizer 存在 |
| 默认 | 声明层存在、强制层不足 | 规则声明但无 hook 强制 |
| 持久化 | 主路径具备、边缘路径缺失 | .chanlun/ 完整，会话关闭/tmp/ 有盲区 |
| 向下 | 流程定义完整、强制层缺失 | sub-swarm-ceremony skill 完整，无五特征验证 hook |
| 无限递归 | **内部矛盾** | "无限"与"3-4层上限"矛盾 |

**整体结论**: 该声明在**声明层/设计层**基本成立，在**执行层/强制层**有显著缺口。核心问题模式一致：声明在 CLAUDE.md/skills/dispatch-dag 中存在，但代码强制（hooks/scripts）覆盖不完整。这正是 spec-execution-gap（036号）的同构模式——声明 != 能力。

---

## 五、修复优先级建议

| 优先级 | 缺口 | 修复方案 | 类型 |
|--------|------|----------|------|
| P0 | G1: 无限递归声明矛盾 | 修改声明，消除内部矛盾 | 声明修正 |
| P0 | G2: DAG 解析器缺失 | 二选一：实现解析器或修改声明 | 架构决策 |
| P1 | P2: 会话关闭持久化 | 添加 SessionEnd hook 或等效机制 | 代码实现 |
| P1 | P6: 不动点假阳性 | 审计 3 个 unresolved 项，补充 verification_hints | 数据修正 |
| P2 | P1: 子蜂群五特征强制 | 在 agent-team-enforce.sh 中添加五特征检查 | hook 增强 |
| P2 | P7: tmp/ 结论结晶 | 在 ceremony 中检测 tmp/ 下有结晶价值的产出 | 流程增强 |
| P3 | P3-P5: 默认/并行/四分法 | 这些属于 LLM 行为层面，hook 能覆盖的有限；rules/ 注入是当前最优解 | 监控增强 |
