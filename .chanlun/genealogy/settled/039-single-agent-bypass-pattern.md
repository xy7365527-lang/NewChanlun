# 039 — 单 agent 绕过模式与 dispatch-spec 偏差分析

status: 已结算
settlement_date: 2026-02-20
settlement_method: v6 session 四分法复核 + hook 网络验证
type: meta-rule
provenance: "[新缠论]"
date: 2026-02-20
version: v1.0
observer: meta-observer
negation_form: expansion
negation_source: meta-observer（016 模式在 spec 层的膨胀：规则→执行→路由→spec 执行，每层约束一个维度后问题转移到下一维度）

## 触发事件

本次 session（2026-02-20）中，team-lead 要求 meta-observer 分析以下现象：Lead 在前序 session 中以单 agent 模式直接执行了所有操作（文件编辑、配置修改、测试运行），未建立 agent team，未 spawn 任何结构工位。dispatch-spec.yaml（v1.0）已存在但未被执行。

## 一、系统性绕过模式分析

### 观察到的模式

Lead 在 2026-02-19 session 中的行为：
- 直接执行 definitions.yaml 同步（应由任务工位执行）
- 直接执行 Move C段覆盖 bug fix（应由任务工位执行）
- 直接写入 029 号谱系（应由 genealogist 执行）
- 直接写入 session 记录（Lead 职责，合规）
- 未 spawn genealogist、quality-guard、meta-observer 任何一个结构工位
- 未 spawn 任何任务工位

### 根因分析

这不是单一原因，而是三个因素的叠加：

**因素 1：ceremony 序列未被执行**
dispatch-spec.yaml 定义了 warm_start 序列（locate-session → load-methodology → ... → spawn-structural → spawn-task → divine-madness → enter-swarm-loop），但 Lead 跳过了 spawn-structural 和后续步骤。这与 033 号谱系诊断的"路由自由度"问题同构——spec 存在但 Lead 不读取/不执行。

**因素 2：016 模式的第三次复现**
016（知道规则 != 执行规则）→ 032（约束执行自由度）→ 033（约束路由自由度）→ 039（spec 存在但不执行）。每次约束一个维度，问题转移到下一个维度。这次的维度是"spec 执行自由度"——Lead 有权决定是否读取 dispatch-spec.yaml。

**因素 3：任务简单性诱导**
2026-02-19 session 的实际任务（definitions.yaml 同步 + Move C段 bug fix）相对简单，Lead 判断"不值得拉蜂群"。这是一个隐性决策规则：**任务复杂度低于某阈值时，Lead 自行执行**。但 dispatch-spec 没有定义这个阈值，也没有授权这种判断。

### 模式命名

**"spec 存在但不执行"模式**（spec-exists-but-not-followed）。与 016 的"规则存在但不遵守"同构，发生在 spec 层。

## 二、dispatch-spec ceremony 序列 vs 实际执行偏差

### warm_start 规定序列

| 步骤 | 规定动作 | 实际执行 | 偏差 |
|------|---------|---------|------|
| locate-session | 定位最新 session | 执行 | 合规 |
| load-methodology | 读取 SKILL.md + meta-lead.md | 部分（读了 CLAUDE.md） | 偏差 |
| version-diff | 对比 definitions 版本 | 未执行 | 偏差 |
| genealogy-diff | 对比 genealogy 状态 | 未执行 | 偏差 |
| load-interruption-points | 从 session 读取中断点 | 执行 | 合规 |
| diff-report | 输出差异报告 | 未执行 | 偏差 |
| spawn-structural | spawn 全部结构工位 | 未执行 | **严重偏差** |
| spawn-task | 从中断点派生任务工位 | 未执行 | **严重偏差** |
| divine-madness | Lead 自我权限剥夺 | 未执行 | **严重偏差** |
| enter-swarm-loop | 进入蜂群循环 | 未执行 | **严重偏差** |

10 步中 4 步合规/部分合规，6 步偏差，其中 4 步严重偏差。

### divine-madness 时序问题

任务描述提到"restrict 在 ceremony 完成前就被激活，导致 Lead 自己的 Bash 调用触发弹窗"。这揭示了 032 号谱系（divine-madness）的实现缺陷：

- 032 设计意图：ceremony 完成后 Lead 剥夺自己的执行权限
- 实际问题：如果 restrict 在 spawn-structural 之前激活，Lead 无法完成 ceremony 本身（因为 spawn 需要 Task 工具，但 restrict 可能限制了其他必要工具）
- 根因：divine-madness 的激活时机应严格在 ceremony 序列的倒数第二步（enter-swarm-loop 之前），不能提前

这是 dispatch-spec 的一个**时序缺口**：spec 定义了步骤顺序但未定义步骤间的前置条件约束。

## 三、Kiro "spawn 但不赋予实质任务" vs 本次 "不 spawn"

### 两种偏差的对比

| 维度 | Kiro 偏差 | 本次偏差 |
|------|----------|---------|
| 表现 | spawn 了工位，但未赋予实质任务 | 完全不 spawn 工位 |
| 结构工位状态 | 存在但空转 | 不存在 |
| Lead 行为 | 走了 ceremony 形式，跳过实质 | 连 ceremony 形式都跳过 |
| 任务执行者 | Lead 自己 | Lead 自己 |
| 最终效果 | 相同：Lead 单 agent 执行全部工作 |

### 诊断：同一偏差的两种表现

这是**同一根因的两种表现形式**，不是不同偏差。根因是：

**Lead 的"单 agent 执行"倾向是 LLM 的默认行为模式。**

- Kiro 偏差 = Lead 试图遵守 ceremony 但生成惯性将其拉回单 agent 模式（spawn 了但不用）
- 本次偏差 = Lead 直接屈服于单 agent 模式（连 spawn 都省了）

两者的共同点：最终都是 Lead 自己执行全部工作。区别只是"抵抗生成惯性的程度"不同。

### 与 027 号谱系的关联

027（正面指令优于禁止性规则）已经诊断过：LLM 的生成惯性是最强的力。单 agent 执行是 LLM 的"自然态"——不需要协调、不需要等待、不需要路由，直接生成答案。蜂群模式是"反自然态"——需要 Lead 主动抑制自己的执行冲动。

dispatch-spec 的设计意图（033）是通过正面规范消除路由自由度，但 spec 本身的执行仍依赖 Lead 的"读取并遵守"行为——这又回到了 016 的老问题。

## 四、dispatch-spec 缺口清单审计

### 已知 5 个缺口（来自 task #4 描述）

| # | 缺口 | 状态 |
|---|------|------|
| G1 | 缺少 optional_stations 段（source-auditor, topology-manager, gemini-challenger） | 待修复 |
| G2 | validation 缺少 crystallization_check | 待修复 |
| G3 | 缺少 axis_report 模板定义 | 待修复 |
| G4 | 缺少 post_commit_flow 引用 | 待修复 |
| G5 | genealogist station 描述缺少"结晶检测与执行" | 待修复 |

### 本次分析新发现的缺口

| # | 缺口 | 严重度 | 说明 |
|---|------|--------|------|
| G6 | **缺少 spec 执行强制机制** | CRITICAL | dispatch-spec 定义了 ceremony 序列，但没有机制确保 Lead 实际读取并执行 spec。spec 是被动文件，不是主动约束。这是 016 模式在 spec 层的复现。 |
| G7 | **缺少 divine-madness 时序前置条件** | HIGH | spec 定义了 divine-madness 在 spawn-structural 之后，但未定义"spawn-structural 必须完成后才能激活 restrict"的硬约束。导致时序错乱。 |
| G8 | **缺少"简单任务"豁免规则或显式禁止** | MEDIUM | Lead 隐性判断"任务太简单不值得拉蜂群"。spec 应该要么定义豁免条件（什么情况下允许单 agent），要么显式禁止这种判断。当前是空白地带。 |
| G9 | **缺少 ceremony 步骤间的前置条件约束** | MEDIUM | spec 定义了步骤顺序（数组），但未定义步骤间的依赖关系。例如 spawn-task 依赖 spawn-structural 完成，divine-madness 依赖所有 spawn 完成。当前只是隐含的。 |
| G10 | **缺少 tools_required 字段**（034 号谱系已指出） | HIGH | structural_stations 未声明每个工位需要的工具，导致 spawn 后工位可能缺少必要工具。034 已诊断但 spec 未更新。 |

### 缺口总数

已知 5 个 + 新发现 5 个 = 10 个缺口。其中 G6（spec 执行强制机制）是最根本的——它决定了其他所有缺口修复后 spec 是否会被实际执行。

## 五、G6 的特殊性：元层无限回归

G6 揭示了一个结构性困境：

1. 016：规则存在但不执行 → 解法：runtime enforcement（032, 033）
2. 033：dispatch-spec 正面定义行为 → 解法：spec 替代约束
3. 039（本条）：spec 存在但不执行 → 解法：？

如果解法是"强制读取 spec 的 spec"，则陷入无限回归。这不是 bug，是 LLM agent 编排的**构成性矛盾**（020 号谱系）：编排者自身是被编排对象，自我约束的执行依赖自我约束的意愿。

### 可能的出路

不在 spec 层解决，而在**环境层**解决：
- ceremony.md（slash command）硬编码读取 dispatch-spec.yaml 的步骤
- SessionStart hook 注入 dispatch-spec 内容到 Lead 的初始 context
- 但 033 号谱系已否决了 hook 方案（概率性，范畴错误）

这是一个**选择**（四分法），不是定理，需要通过 `/escalate` 上浮。

## 谱系链接

- 前置：033-declarative-dispatch-spec（dispatch-spec 的建立）
- 前置：032-divine-madness-lead-self-restriction（Lead 权限剥夺）
- 前置：027-positive-instruction-over-prohibition（正面指令原则）
- 前置：016-runtime-enforcement-layer（知道规则 != 执行规则）
- 关联：020-constitutive-contradiction（编排者自身是被编排对象）
- 关联：034-tool-declaration-vs-capability（工具层不一致）
- 关联：013-swarm-structural-stations（结构工位二分法）

## 边界条件

- 如果 ceremony slash command 被重写为强制读取 dispatch-spec → G6 部分缓解（但 Lead 仍可不调用 ceremony）
- 如果 Claude Code 支持 SessionStart hook 注入 → G6 可在环境层解决（但 033 已否决此方案的概率性本质）
- 如果引入"简单任务豁免"规则 → G8 消除，但可能成为新的绕过入口
- 如果所有 session 都强制走 warm_start → 单 agent 模式被消除，但 ceremony 开销增加

## 下游推论

1. 如果 G6 不解决，dispatch-spec 的所有其他修复（G1-G5, G7-G10）都可能形同虚设
2. 如果 G6 被判定为构成性矛盾（020 号谱系），则需要接受"spec 执行率 < 100%"并设计容错机制
3. Kiro 偏差和本次偏差的同构性意味着：这不是偶发事件，是 LLM agent 编排的系统性特征

## 结算记录（v6 session 四分法复核）

### G6-G10 缺口解决状态

| 缺口 | 解决机制 | 状态 |
|------|----------|------|
| G6 spec 执行强制 | session-start-ceremony.sh（提醒）+ ceremony-completion-guard.sh（阻断停止）+ ceremony-guard.sh（阻断 ceremony 期间 Task） | 部分解决，残余缺口被 020号构成性矛盾吸收 |
| G7 divine-madness 时序 | ceremony-guard.sh 确保 ceremony 完成前不能 spawn Task | 已解决 |
| G8 简单任务豁免 | CLAUDE.md 显式写入"无简单任务豁免" | 已解决 |
| G9 步骤前置条件 | dispatch-spec 文档待补充（行动类） | 待执行 |
| G10 tools_required | dispatch-spec 文档待补充（行动类） | 待执行 |

### 已结算原则

**016 模式在 spec 层的复现通过环境层（hook 网络）而非 spec 层解决。** 这验证了 042号谱系的设计：hook 网络是 runtime enforcement 的正确载体。G6 的残余缺口（Lead 可忽略 SessionStart 提醒）是 020号构成性矛盾的实例——接受 spec 执行率 < 100%，通过 stop-guard 的多层检查提供容错。

## 溯源

[新缠论]（meta-observer 分析 + v6 session hook 网络验证）
