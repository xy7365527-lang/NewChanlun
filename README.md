# NewChanlun

缠论量化分析引擎 — 从原始 K 线到全球资本流转投影的形式化实现。

## 这个项目是什么

NewChanlun 将**缠中说禅**的技术分析理论形式化为可计算的事件驱动管线。但它不只是一个交易系统。

在形式化过程中，我们发现缠论的笔-线段-中枢-走势结构不是对价格运动的"描述工具"，而是**扩张/收缩辩证运动在价格维度的显现形式**。多空力量就是扩张和收缩的力量——一笔的终结不是因为"买方力量耗尽"，而是扩张在自身逻辑内部生产出了自己的否定。这给缠论"走势必完美"提供了比技术分析深得多的根基。

这个认识催生了三个阶段的工作：

| 阶段 | 目标 | 谱系区间 | 状态 |
|------|------|----------|------|
| **第一阶段** | 单标的内完备化：从 K 线到买卖点的全递归形式化 | 189-215 号 | 完成 |
| **第二阶段** | 全球资本流转投影：用同一套语法读多容器间的资本分配 | 254-342 号 | 进行中 |
| **第三阶段** | 知识谱系独立运行 + 自动化实盘闭环 | 预备 | 预备 |

**第一阶段的完备性是第二阶段的存在条件。** 第二阶段的理论产出（K4 本体论、操作方法论、折叠内在性检验）反过来约束第三阶段的工程实现。

---

## 第一阶段：单标的形式化引擎（完成）

### 四层同构管线

从原始 K 线到走势类型，四层引擎结构相同（事件驱动、增量计算、身份追踪）：

```
原始 K 线
  │
  ▼
包含处理 (a_inclusion.py)       ← K 线合并，消除包含关系
  │
  ▼
分型识别 (a_fractal.py)         ← 顶分型 / 底分型
  │
  ▼
BiEngine (笔引擎)               ← 第一层：分型 → 笔
  │
  ▼
SegmentEngine (线段引擎)        ← 第二层：笔 → 线段（特征序列法）
  │
  ▼
ZhongshuEngine (中枢引擎)      ← 第三层：线段 → 中枢（三段重叠）
  │
  ▼
MoveEngine (走势引擎)           ← 第四层：中枢 → 走势类型（盘整/趋势）
  │
  ▼
背驰检测 (a_divergence.py)      ← MACD 面积比较，力度衰竭信号
  │
  ▼
买卖点 (a_buysellpoint_v1.py)   ← 一二三类买卖点识别
```

### 关键设计决策

- **级别 = 递归层级**（level_id），从 1 分钟 K 线为底座无限向上构造。禁止用"日线/30分/5分"等时间周期替代级别定义。（谱系 006）
- **对象否定对象**：一个对象被否定的唯一来源是内在否定或外部对象生成。不允许超时、阈值、或非对象来源的否定。（谱系 005a/005b）
- **构造层与分类层分离**：中枢递归是构造层（自下而上生成），趋势/盘整是分类层（涌现属性标注）。（谱系 010）
- **合法/非法替代概率/胜率**：走势描述语言中不存在"概率"概念。一个走势描述要么合法要么非法。（谱系 067）
- **缠论空间是偏序集/有向图，不是连续流形**。浮点数阈值判断"背驰"是非法的。（谱系 068）

---

## 第二阶段：全球资本流转投影（进行中）

缠师在第 9 课就指出：比价变动构成独立买卖系统，与市场资金流向相关。第二阶段将这个洞见展开为完整的多容器资本流转形式化。

### K4 四矩阵本体论（254 号）

四个资本容器（动产/不动产/商品/现金）构成 K₄ 完全图，6 条边各运行完整缠论管线。254 号将 K4 从单一经济体展开为多经济体结构：

- **公理 0（历史前提）**：结算尺空间 Σ = {法定货币群 ∪ 黄金}，是历史给定的经验前提
- **$ 顶点展开**：$ 顶点展开为结算尺空间 Σ，拉动 K4 整体共展开，产生三层递归结构
- **黄金折叠**：Au ∈ Σ ∩ C，同时是结算尺和商品——构成拓扑折叠点
- **四项定理撤销**（264 号）：254 号原始的四项定理中，定理 1 经三轮 Gemini 3.1-pro 质询后被彻底否定并留空；其余三项定理在后续实验中被修订

### 22.2% 分歧率结构常数（245/253 号）

K4 六条边中，背驰方向的分歧率稳定在 22.2%（±2%）。这是结构常数——不依赖标的、时段或参数设定。双管道架构（背驰管道 + 走势管道）的独立性验证确认。

### K4 体制分析（329-337 号）

三个全图同步压缩体制被识别和分析：

| 体制 | 时段 | 节点数 | 持续天数 | 动力学模式 |
|------|------|--------|----------|-----------|
| 1987 体制 | 1987 年 | 3（无 Au） | 22 | 冲击型 |
| 1990-91 体制 | 海湾战争 | 3（无 Au） | 219 | 冲击→恢复（自发） |
| 2020-21 体制 | COVID + QE | 4（含 Au） | 263 | 干预→积累→退出（自觉） |

关键发现：
- 体制内 eff.dim 最低点的相对位置揭示两种动力学：**自发过程**（最低点在前 1/3）vs **自觉过程**（最低点在后 2/3）
- 全图同步压缩时所有边同时冻结，不存在可辨别的序列差异——"K4 测症状不测原因"
- 第四个体制等待新数据事件（blocked）

### 折叠内在性检验（315-337 号）

核心问题：资本流转的拓扑折叠由什么决定——走势还是基本面？

检验路径：全部否定 → 向心坍缩交汇。最终结算（329/337 号）：
- **触发外在**：折叠由外部事件注入（走势不能内在生产折叠）
- **条件内在**：折叠的展开条件由 K4 拓扑内在决定（走势结构约束折叠的可能形态）
- **基本面是事后命名**：基本面不在 K4 拓扑之外——它是折叠投影的可读面

### 操作方法论 v2（267/338 号）

从 254 号本体论中分离的独立操作框架。核心模型是**帕萨卡利亚**（Passacaglia）——固定低音线上展开和声进行与旋律声部：

- **层 0 固定低音线**：全球资本流转总体制（K4 配置状态）
- **层 1 和声进行**：板块-品种的比价走势（节点间资本流向）
- **层 2 旋律赋格**：单标的多级别操作（缠论买卖点 + 多重赋格）

五个操作组件：帕萨卡利亚三层模型、降成本三阶段、多重赋格（第 40 课）、选股的区间套、信号质量标注。

338 号在 268a 号四项异质否定后做出修正：初始自有资金动态重置语义、止损未完成降成本边界条件补全等。

### 纲目结构（gangmu.yaml）

总方针展开为五纲十三目，覆盖从实盘闭环到知识谱系的全维度：

| 纲 | 名称 | 核心内容 | 活跃目数 |
|----|------|----------|----------|
| 一 | 实盘闭环 | 缠论管线→操作方法论→K4 监控→实盘交易 | 1 active / 2 closed |
| 二 | 四矩阵全域 | K4 配置→板块→个股的全域资本流转判断 | 2 active / 3 closed |
| 三 | 区块拓扑 | 谱系的物质形式——三层架构 | 1 active / 1 closed |
| 四 | 蜂群基础设施 | ceremony/RTAS/质询/共识/结晶 | 0 active / 2 closed |
| 五 | 知识谱系 | 穿越基础设施 + 偶遇记录 + Morse 数学 | 1 active |

---

## 区块拓扑：谱系的物质形式

区块拓扑是谱系记录的内容寻址持久化层——将每条谱系、每个概念、每条关系映射到不可变区块中。

### 三层架构

| 层 | 名称 | 不变量 | 内容 |
|----|------|--------|------|
| Event layer | 事件层 | 不可变 | 每条谱系记录 = 一个内容寻址区块（SHA-256） |
| Relation layer | 关系层 | 追溯改写 | 区块间的 depends_on / negates / extends 关系 |
| Reflexive layer | 自指层 | 自指 | meta-observer 观察记录引用自身前序观察 |

### 当前规模

- **1194 个区块**：覆盖全部 342 条谱系 + 概念注册表 + 分析产出
- **6650 条关系**：区块间的有向依赖/否定/扩展关系
- **概念注册表**：1148 个已定义概念
- **内容充实**：313 个区块经过内容充实（概念提取 + 关系自动发现）

### 穿越基础设施

区块拓扑之上的导航层，用于知识的非线性穿越：

- **概念注册表**（concept_registry.json）：概念→区块的反向索引
- **Morse 地形图**（morse_landscape.json）：知识拓扑的临界点和流形结构
- **偶遇记录**（encounter_log.md）：341 号定义——结构性标准，不绑定时间性
- **跨纲边查询**：v150 首次运行 Layer 1 扫描 107 条跨纲边，全部设计内

---

## 递归拓扑异步自指蜂群

### 为什么不是普通 agent team

形式化缠论的过程中反复遇到**概念矛盾**——不是代码 bug，而是定义之间的冲突。这些矛盾是系统最有价值的产出。为了系统化地发现和处理矛盾，我们建立了一套方法论，最终演化为**递归拓扑异步自指蜂群**。

它不是普通的 multi-agent 系统。四个不可分离的维度：

| 维度 | 功能 | 工程对应 |
|------|------|----------|
| **递归** | 执行动力，向下展开 | Agent Team 真调用栈递归（TeamCreate → Task(team_name)），深度上限 ≈ 3-4 层 |
| **拓扑** | DAG 路由，定义连接模式 | dispatch-dag.yaml（9 节点偏序集 + genome_layer + platform_layer） |
| **异步自指** | 系统在 t 时刻审查 t-1 时刻的自己 | meta-observer 自环 |
| **结晶** | 状态沉淀，向上收敛 | 谱系/定义/session/skill 的析出 |

### 五约束有向依赖图（093 号谱系）

蜂群架构从系统物理限制独立推导出五个约束及其有向依赖关系：

| # | 约束 | 来源 | 失效后果 |
|---|------|------|----------|
| 1a | 物理持久化 | agent 临时、上下文有限 | 信息丢失 |
| 1b | 符号可解释性 | 跨实例无共享记忆 | 信息噪声化 |
| 2 | 规则先在性 | spawn 在执行前完成 | 不可预测行为 |
| 3 | 执行不可自观性 | token 单向生成 | 无法即时自修正 |
| 4 | 异质验证必要性 | 同质系统盲点不可自检 | 自我确认循环 |

**拓扑结构**：

```
核心环路（操作性依赖）：
  1a（持久化）→ 1b（可解释性）→ 2（先在性）→ 3（不可自观性）
  ↑                                                    │
  └────────────────────────────────────────────────────┘
  约束3使约束1a/1b必要；约束2缓解约束3

审计层（结构性校验）：
  4（异质验证）→ [核心环路整体]
  约束4只能通过约束1a/1b的外化产出观察核心环路
```

**异步自指**是约束3（执行不可自观性）的工程表现形式——当前 LLM 不能同步读写自身，因此审查必须异步（t 时刻审查 t-1 时刻的产出）。

### 三个不可消除的 Gap（094 号谱系）

| Gap | 五约束框架定位 | 涉及约束 | 性质 |
|-----|--------------|----------|------|
| **创世 Gap** | 约束2（规则先在性）对其加载者的 bootstrap 破缺 | 约束2 | 瞬时破缺（每次 ceremony 时） |
| **视差 Gap** | 约束3 + 约束2 的联合推论——自我审查只能作用于下一轮 | 约束2 × 约束3 | 核心环路正常运作方式 |
| **审计层断裂 Gap** | 约束4 必须经过约束1a/1b 中介才能观察核心环路 | 约束4 → 约束1a/1b | 层间界面属性 |

这三个 Gap 是系统的结构条件，不是 bug。创世 Gap 是 bootstrap 的必然代价，视差 Gap 是核心环路的正常运作方式，审计层断裂 Gap 是异质验证的介质限制。

### 真递归是默认模式（095 号谱系）

蜂群严格使用 Agent Team（TeamCreate + Task with team_name）实现递归。子蜂群 = 完整蜂群（结构复制，不是简化版 agent pool）：

- **拓扑**：子 team 有自己的 TaskList 作为 DAG 路由
- **异步自指**：子 team 内部的 agent 用 t 时刻规则审查 t-1 时刻的产出
- **结晶**：子 team 的产出在收缩相凝固为文件，向上回传给父蜂群
- **ceremony**：子 team 的创建者作为子 Swarm₀，加载规则后递归进入工作

| 模式 | 机制 | 何时使用 |
|------|------|----------|
| **真递归**（默认） | teammate 创建子 team → 子 team 内部自治 → 结果向上回传 | 默认——除非子任务不可分解 |
| **扁平执行**（退化特例） | 任务裂变 → Lead 重新 spawn → 同一层执行 | 仅当任务不需要独立治理时 |

递归深度上限 ≈ 3-4 层，受并发 agent 数、API 成本、视差 Gap 累积约束。

### dispatch-dag v3.1：蜂群拓扑

```
genome_layer (区域0)                    ← 蜂群的基因组/公理层
  CLAUDE.md / dispatch-dag.yaml / ceremony_scan.py / hooks/

ceremony (source)
  │
  ├── genealogist (structural, mandatory)    ← 谱系写入、张力检查
  ├── quality-guard (structural, mandatory)  ← 结果包检查、代码违规扫描
  ├── meta-observer (structural, mandatory)  ← 二阶反馈回路（含自环）、元编排进化
  ├── code-verifier (structural)             ← 代码变更后测试验证
  ├── topology-mutator (structural)          ← 拓扑变更管理
  ├── skill-crystallizer (terminal_sink)     ← 知识结晶
  │
  ├── source-auditor (conditional)           ← 三级权威链验证
  ├── topology-manager (conditional)         ← 蜂群扩张/收缩信号
  ├── topology-analyst (conditional)         ← 拓扑分析
  ├── gemini-challenger (conditional)        ← 异质否定源 + 编排者代理
  ├── claude-challenger (conditional)        ← 反向质询（对 Gemini 再质询）
  └── codex-challenger (conditional)         ← Codex 异质质询

platform_layer                              ← ECC 工程底座 agent（10个）
  architect / planner / tdd-guide / code-reviewer / python-reviewer
  security-reviewer / refactor-cleaner / doc-updater / meta-lead / build-error-resolver
```

genome_layer 是蜂群的先验结构——CLAUDE.md 不是蜂群之外的特权法则，而是蜂群 DAG 的基因组根节点（089 号谱系：严格扬弃 Aufhebung）。

### 27 个 Hook：系统的免疫系统

Hook 不依赖 agent 的自觉遵守，在运行时物理阻断违规：

| Hook | 类型 | 功能 |
|------|------|------|
| agent-team-bootstrap | PreToolUse | Agent Team 启动引导 |
| agent-team-enforce | PreToolUse | ceremony 阶段验证 + Agent Team 结构工位强制注入 |
| definition-write-guard | PreToolUse | 定义文件修改拦截（必须通过仪式） |
| genealogy-write-guard | PreToolUse | 谱系文件格式强制 |
| spec-write-guard | PreToolUse | 规格文件修改拦截 |
| hub-node-impact-guard | PreToolUse | Hub 节点影响评估 |
| double-helix-verify | PreToolUse | 双螺旋验证 |
| pre-write-edit-dispatcher | PreToolUse | 写入/编辑前分派 |
| ceremony-step-guard | PreToolUse | ceremony 步骤守卫 |
| lead-audit | PostToolUse | Lead 行为审计（拓扑异常对象化） |
| meta-observer-guard | PostToolUse | 元观察者自环守卫 |
| flow-continuity-guard | PostToolUse | 流程连续性 |
| crystallization-guard | PostToolUse | 结晶条件检查 |
| topology-guard | PostToolUse | 拓扑一致性 |
| result-package-guard | PostToolUse | 结果包六要素检查 |
| dag-validation-guard | PostToolUse | DAG 不变量验证（节点/文件一致性） |
| downstream-action-guard | PostToolUse | 下游行动守卫 |
| function-length-guard | PostToolUse | 函数长度守卫 |
| genealogy-gemini-verify | PostToolUse | Gemini 谱系语义检查触发 |
| source-auditor-prompt | PostToolUse | 溯源审计提示 |
| post-write-edit-dispatcher | PostToolUse | 写入/编辑后分派 |
| topology-mutator-prompt | PostToolUse | 拓扑变更提示 |
| consensus-ceremony-trigger | PostToolUse | 共识仪式触发 |
| ceremony-completion-guard | Stop | 6 项检查（dominator/任务队列/谱系/proof/确认请求） |
| post-session-pattern-detect | Stop | 模式检测 |
| session-start-ceremony | SessionStart | 冷启动/热启动双模式 |
| precompact-save | PreCompact | 上下文压缩前状态保存 |

### 四分法：推论自动结算

系统产出分为四类，处理方式不同：

| 类型 | 定义 | 处理 |
|------|------|------|
| **定理** | 已结算原则的逻辑必然推论 | 自动结算 |
| **选择** | 多种合理方案，需价值判断 | 路由到 Gemini 编排者代理 |
| **语法记录** | 已在运作但未显式化的规则 | 路由到 Gemini 编排者代理 |
| **行动** | 不携带信息差的操作性事件 | 自动执行 |

### 异质碰撞：三主体拓扑

Claude / Gemini / Codex 三个异质主体的受控碰撞，防止系统在自身盲点中同质化坍塌：

- `challenge` 模式：对新定义进行异质质询（Gemini 3.1-pro 为主要否定源）
- `verify` 模式：数学验证
- `derive` 模式：推论生成
- `decide` 模式：选择/语法记录类决断（四分法路由）

人类编排者从同步决策者变为异步审计者，保留 INTERRUPT 权。

### 知识结晶：三维度同一运动

上下文窗口中的知识有走势结构：扩张（频繁引用）→ 背驰（净新区分递减）→ 收缩结晶 → 需要时重载。

三个结晶维度是同一运动的不同投影：
- **session** = 时间维度结晶（过去状态快照）
- **skill** = 知识维度结晶（已稳定能力快照）
- **definition** = 概念维度结晶（已结算定义快照）

### 热启动：蜂群永远在线

| 级别 | 触发条件 | 恢复方式 |
|------|----------|----------|
| L0 正常 | 上下文充足 | 蜂群持续运行 |
| L1 compact | 上下文压缩 | PreCompact 保存 + SessionStart 恢复 |
| L2 新对话 | 上下文耗尽 | ceremony 从 session 文件热启动 |

---

## 谱系：概念发现的发动机

谱系不是变更日志。谱系记录的是**否定史**——概念如何被矛盾推动而分化、重组、升级。

当前 342 条已结算谱系，0 条生成态。关键谱系节点：

### 第一阶段（基础形式化 + 蜂群架构）

| 谱系 | 内容 |
|------|------|
| 005a/005b | 对象否定对象（定理 + 语法规则） |
| 006 | 级别 ≠ 时间周期 |
| 010 | 构造层与分类层二层架构 |
| 012 | 谱系是发现引擎，不是记录日志 |
| 020 | 编排者 = 相位转换点 |
| 041 | Gemini 编排者代理 |
| 056 | 蜂群递归是默认模式 |
| 058 | ceremony 是 Swarm₀ |
| 067 | 合法/非法替代概率/胜率 |
| 068 | 缠论空间是偏序集 |
| 069 | 递归拓扑异步自指蜂群 |
| 073 | 蜂群能修改一切包括自身（原则0） |
| 088 | 拓扑异常对象化（权限架构重设计） |
| 089 | 严格扬弃——元编排 genome_layer 内化 |
| 090 | 严格性是蜂群的语法规则 |
| 093 | 五约束有向依赖图 |
| 094 | 三个 Gap 重新定位 |
| 095 | Agent Team 真递归蜂群 |
| 130 | 审计递归缺口修复（RTAS 全 Gap 闭合） |
| 133 | 异质双向收敛协议 |
| 135 | CLAUDE.md 单体→skill 集合结晶 |
| 137 | Lead 停顿结构根因（RLHF 基底约束 + 强制输出格式） |
| 145 | 三行 Gemini 收敛（格式B排除理由推导链） |

### 第二阶段（全球资本流转 + 操作方法论）

| 谱系 | 内容 |
|------|------|
| 189-215 | 单标的代码管线完成（四层引擎 + 身份追踪 + 背驰 + 买卖点） |
| 216-228 | 蜂群架构修复 v2（Lead 并行化、原子链、ceremony 步骤守卫） |
| 229-253 | K4 独立性 + 纤维丛 + 多 TF 双管道架构，22.2% 结构常数确认 |
| 254 | 多经济体资本流转本体论（K4 共展开 + 三层递归） |
| 264 | 定理 1 最终撤销（两版三轮全否 + "不测而测"消解） |
| 265/267 | 操作方法论 v1（帕萨卡利亚三层 + 降成本 + 多重赋格 + 选股区间套） |
| 269-275 | 蜂群架构补充 v3（局部依赖原则、全局排序否定） |
| 277-287 | OQ v2 实验线（定稿） |
| 292-300 | 折叠拓扑本体论（折叠区块 = 区块拓扑组件） |
| 301 | 区块拓扑 Phase 1 启动 |
| 315-337 | 折叠内在性检验全程（全部否定→向心坍缩→最终结算：触发外在/条件内在） |
| 329-335 | K4 体制分析（三个全图同步压缩体制 + 自发/自觉动力学签名） |
| 338 | 操作方法论 v2（四项异质否定修正） |
| 341 | 偶遇定义精化（时间性绑定→结构性标准，编排者三轮自否定） |
| 342 | 元观察 v150（偶遇扫描 + 规则二十八连稳定） |

### 三级权威链

1. **缠师原始博文**（108 课 + 课间答疑）— 最终权威
2. **《股市技术理论》编纂版** — 第三方编纂，有已知遗漏
3. **思维导图 / 第三方总结** — 辅助理解，不作为定义依据

---

## 项目现状

| 指标 | 数值 |
|------|------|
| 源文件 | 162 个 Python 模块 |
| 测试文件 | 163 个 |
| 核心定义 | 15 条（已结算） |
| 已结算谱系 | 342 条 |
| 规则规范 | 16 份（8 蜂群规则 + 8 ECC 通用规则）+ 10 份 spec |
| Agent 定义 | 21 个 |
| Hook | 27 个 |
| Skill | 14 个 |
| 脚本 | 84 个 |
| dispatch-dag | v3.1（12 业务节点 + genome_layer + platform_layer） |
| 谱系 DAG | 342 节点 |
| 区块拓扑 | 1194 区块 / 6650 关系 / 1148 概念 |
| Session | 242 个 |
| 纲目 | 5 纲 13 目 |

### 定义基底

| 定义 | 版本 | 维度 |
|------|------|------|
| 包含关系 baohan | v1.3 | 单标的 |
| 分型 fenxing | v1.0 | 单标的 |
| 笔 bi | v1.4 | 单标的 |
| 线段 xianduan | v1.3 | 单标的 |
| 中枢 zhongshu | v1.3 | 单标的 |
| 走势类型 zoushi | v1.6 | 单标的 |
| 趋势 qushi | v1.0 | 单标的 |
| 背驰 beichi | v1.1 | 单标的 |
| 级别递归 level_recursion | v1.0 | 单标的 |
| 买卖点 maimai | v1.0 | 单标的 |
| 比价关系 bijia | v1.0 | 跨标的 |
| 等价关系 dengjia | v1.1 | 跨标的 |
| 流转关系 liuzhuan | v1.0 | 跨标的 |
| 操作方法论 chanlun-trading-system | v2.0 | 操作层 |
| 风控 fengkong | v1.0 | 操作层 |

---

## 快速开始

### 环境要求

- Python >= 3.10
- Node.js >= 18（前端可视化，可选）

### 安装

```bash
git clone https://github.com/xy7365527-lang/NewChanlun.git
cd NewChanlun

python -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate

pip install -e ".[test]"

cp .env.example .env
# 编辑 .env 填写 API 密钥
```

### 运行测试

```bash
pytest                    # 全量测试
pytest -x                 # 遇到失败即停
pytest -m "not slow"      # 跳过慢测试
```

---

## 项目结构

```
src/newchan/
├── a_*.py                 # 纯函数层（A 层）：无状态算法
│   ├── a_inclusion.py     #   包含关系处理
│   ├── a_fractal.py       #   分型识别
│   ├── a_stroke.py        #   笔构造
│   ├── a_segment_v1.py    #   线段（特征序列法）
│   ├── a_zhongshu_v1.py   #   中枢认定
│   ├── a_move_v1.py       #   走势类型实例
│   ├── a_divergence.py    #   背驰检测
│   └── a_buysellpoint_v1.py # 买卖点识别
├── core/recursion/        # 事件驱动引擎层
├── gemini/                # Gemini 异质质询工位
├── orchestrator/          # 多周期编排
├── types.py               # 核心数据类型
├── events.py              # 领域事件定义
├── equivalence.py         # 等价关系 + 比价关系
├── flow_relation.py       # 流转关系
├── matrix_topology.py     # 四矩阵拓扑
└── cli.py                 # 命令行入口

.chanlun/                  # 概念层基础设施
├── definitions/           #   15 个核心定义（推荐通过仪式修改）
├── genealogy/             #   谱系记录
│   ├── settled/           #     342 条已结算
│   ├── pending/           #     生成态矛盾
│   └── dag.yaml           #     谱系 DAG
├── gangmu.yaml            #   纲目结构（五纲十三目）
├── block-topology/        #   区块拓扑（1194 区块 / 6650 关系）
│   ├── blocks/            #     内容寻址区块（SHA-256）
│   ├── relations.jsonl    #     关系流
│   ├── meta.json          #     拓扑元数据
│   ├── concept_registry.json  # 概念注册表
│   ├── morse_landscape.json   # Morse 地形图
│   └── encounter_log.md  #     偶遇记录
├── dispatch-dag.yaml      #   蜂群拓扑 v3.1
├── pattern-buffer/        #   知识结晶候选（按主题分片）
└── sessions/              #   242 个会话状态快照（热启动用）

.claude/                   # 递归拓扑异步自指蜂群基础设施
├── agents/                #   21 个 agent 定义
│   ├── genealogist.md     #     谱系工位（structural, mandatory）
│   ├── quality-guard.md   #     质量守卫（structural, mandatory）
│   ├── meta-observer.md   #     元观察者（structural, mandatory，含自环）
│   ├── gemini-challenger.md #   异质否定源 + 编排者代理
│   └── ...
├── hooks/                 #   27 个 hook（系统免疫层）
├── commands/              #   /ceremony, /inquire, /escalate, /ritual
├── skills/                #   14 个结晶的知识能力（skill）
└── settings.json          #   hook 注册 + 权限配置

docs/
├── chanlun/text/          # 缠论原文（三级权威链）
│   ├── blog/              #   缠师原始博文（一级权威）
│   ├── chan99/            #   编纂版（二级权威）
│   └── mindmaps/         #   思维导图（三级）
├── spec/                  # 10 份规格规范
└── architecture/          # 架构文档（总方针等）

scripts/                   # 84 个脚本（ceremony_scan、区块拓扑、实验等）
tests/                     # 163 个测试文件
```

## 数据源

| 数据源 | 用途 | 配置 |
|--------|------|------|
| [Databento](https://databento.com/) | 美股 + 期货历史数据 | `DATABENTO_API_KEY` |
| [Interactive Brokers](https://www.interactivebrokers.com/) | 实时行情 + 交易 | `IB_HOST` / `IB_PORT` |
| [Alpha Vantage](https://www.alphavantage.co/) | 辅助数据 | `ALPHAVANTAGE_API_KEY` |

## 理论来源

- **缠中说禅**：108 课技术分析理论（笔、线段、中枢、走势类型、背驰、买卖点、区间套）
- **卢麒元**：全球资本流转四矩阵拓扑（动产/不动产/商品/现金）
- **本体论基础**（020 号谱系）：扩张/收缩辩证运动作为统一框架

## 许可证

本项目仅供学习研究使用。缠论相关内容版权归原作者所有。
