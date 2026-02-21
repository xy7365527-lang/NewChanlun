# NewChanlun

缠论量化分析引擎 — 从原始 K 线到全球资本流转投影的形式化实现。

## 这个项目是什么

NewChanlun 将**缠中说禅**的技术分析理论形式化为可计算的事件驱动管线。但它不只是一个交易系统。

在形式化过程中，我们发现缠论的笔-线段-中枢-走势结构不是对价格运动的"描述工具"，而是**扩张/收缩辩证运动在价格维度的显现形式**。多空力量就是扩张和收缩的力量——一笔的终结不是因为"买方力量耗尽"，而是扩张在自身逻辑内部生产出了自己的否定。这给缠论"走势必完美"提供了比技术分析深得多的根基。

这个认识催生了两个阶段的工作：

| 阶段 | 目标 | 状态 |
|------|------|------|
| **第一阶段** | 单标的内完备化：从 K 线到买卖点的全递归形式化 | 基本完成 |
| **第二阶段** | 全球资本流转投影：用同一套语法读多容器间的资本分配 | 进行中 |

**第一阶段的完备性是第二阶段的存在条件。**

---

## 第一阶段：单标的形式化引擎

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

## 第二阶段：全球资本流转投影

缠师在第 9 课就指出：比价变动构成独立买卖系统，与市场资金流向相关。

1. **比价 K 线**：A/B 直接除，产出新 K 线序列，运行同一套完整语法
2. **资本流转语义**：比价一笔向上 = 资本从 B 流向 A。中枢 = 流转暂时均衡。背驰 = 流转动力衰竭
3. **四矩阵拓扑**：四个资本容器（动产/不动产/商品/现金）构成 K₄ 完全图，6 条边各运行完整管线
4. **流转关系**：对每个顶点聚合 3 条边方向，≥2 条同向 = 共振。Σ(net) = 0 = 守恒约束
5. **区间套下钻**：四矩阵比价 → 角内品种 → 具体合约。同一语法，不同分辨率

---

## 递归拓扑异步自指蜂群

### 为什么不是普通 agent team

形式化缠论的过程中反复遇到**概念矛盾**——不是代码 bug，而是定义之间的冲突。这些矛盾是系统最有价值的产出。为了系统化地发现和处理矛盾，我们建立了一套方法论，最终演化为**递归拓扑异步自指蜂群**。

它不是普通的 multi-agent 系统。四个不可分离的维度：

| 维度 | 功能 | 工程对应 |
|------|------|----------|
| **递归** | 执行动力，向下展开 | 任何 teammate 面对可分解子任务时 spawn 子蜂群，无限递归 |
| **拓扑** | DAG 路由，定义连接模式 | dispatch-dag.yaml（9 节点偏序集 + genome_layer + platform_layer） |
| **异步自指** | 系统在 t 时刻审查 t-1 时刻的自己 | meta-observer 自环 |
| **结晶** | 状态沉淀，向上收敛 | 谱系/定义/session/skill 的析出 |

### 两个不可消除的 Gap

- **创世 Gap**：Swarm₀（ceremony）制定拓扑规则但自身不受拓扑约束（bootstrap 必然破缺）
- **视差 Gap**：系统在 t 时刻只能审查 t-1 时刻的自己（异步自指的结构条件）

这两个 Gap 是系统的结构条件，不是 bug。

### dispatch-dag v3.1：蜂群拓扑

```
genome_layer (区域0)                    ← 蜂群的基因组/公理层
  CLAUDE.md / dispatch-dag.yaml / ceremony_scan.py / hooks/

ceremony (source)
  │
  ├── genealogist (dominator, mandatory)     ← 谱系写入、张力检查
  ├── quality-guard (dominator, mandatory)   ← 结果包检查、代码违规扫描
  ├── meta-observer (dominator, mandatory)   ← 二阶反馈回路、元编排进化
  ├── code-verifier (structural)             ← 代码变更后测试验证
  ├── skill-crystallizer (terminal_sink)     ← 知识结晶
  │
  ├── source-auditor (conditional)           ← 三级权威链验证
  ├── topology-manager (conditional)         ← 蜂群扩张/收缩信号
  ├── gemini-challenger (conditional)        ← 异质否定源 + 编排者代理
  └── claude-challenger (conditional)        ← 反向质询（对 Gemini 再质询）

platform_layer                              ← ECC 工程底座 agent（9个）
  architect / planner / tdd-guide / code-reviewer / python-reviewer
  security-reviewer / refactor-cleaner / doc-updater / meta-lead
```

genome_layer 是蜂群的先验结构——CLAUDE.md 不是蜂群之外的特权法则，而是蜂群 DAG 的基因组根节点（089号谱系：严格扬弃 Aufhebung）。

### fractal_template：递归继承

子蜂群通过 fractal_template 继承完整拓扑结构：
- 三个 dominator 节点自动继承（mandatory）
- parent_interface 定义父子蜂群间的验证/谱系/元观察通道
- 递归深度无人为限制，由 topology-manager 的收敛信号自然终止

### 21 个 Hook：系统的免疫系统

Hook 不依赖 agent 的自觉遵守，在运行时物理阻断违规：

| Hook | 类型 | 功能 |
|------|------|------|
| ceremony-guard | PreToolUse | ceremony 阶段 Task spawn 验证 |
| definition-write-guard | PreToolUse | 定义文件修改拦截（必须通过仪式） |
| genealogy-write-guard | PreToolUse | 谱系文件格式强制 |
| spec-write-guard | PreToolUse | 规格文件修改拦截 |
| hub-node-impact-guard | PreToolUse | Hub 节点影响评估 |
| double-helix-verify | PreToolUse | 双螺旋验证 |
| lead-audit | PostToolUse | Lead 行为审计（拓扑异常对象化） |
| team-structural-inject | PostToolUse | TeamCreate 后动态注入结构工位要求 |
| recursive-guard | PostToolUse | 递归 spawn 守卫 |
| flow-continuity-guard | PostToolUse | 流程连续性 |
| crystallization-guard | PostToolUse | 结晶条件检查 |
| topology-guard | PostToolUse | 拓扑一致性 |
| result-package-guard | PostToolUse | 结果包六要素检查 |
| dag-validation-guard | PostToolUse | DAG 不变量验证（节点/文件一致性） |
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

### 异质碰撞：Gemini 编排者代理

Gemini（异质 agent）的受控碰撞，防止系统在自身盲点中同质化坍塌：
- `challenge` 模式：对新定义进行异质质询
- `verify` 模式：数学验证
- `derive` 模式：推论生成
- `decide` 模式：选择/语法记录类决断

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

当前 100 条已结算谱系，0 条生成态。关键谱系节点：

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

### 三级权威链

1. **缠师原始博文**（108 课 + 课间答疑）— 最终权威
2. **《股市技术理论》编纂版** — 第三方编纂，有已知遗漏
3. **思维导图 / 第三方总结** — 辅助理解，不作为定义依据

---

## 项目现状

| 指标 | 数值 |
|------|------|
| 源文件 | 106 个 Python 模块 |
| 测试文件 | 100 个 |
| 核心定义 | 14 条（12 已结算 + 2 基础） |
| 已结算谱系 | 100 条 |
| 规则规范 | 10 份 |
| Agent 定义 | 19 个 |
| Hook | 21 个 |
| dispatch-dag | v3.1（9 业务节点 + genome_layer + platform_layer） |
| 谱系 DAG | 100 节点 / 445 边 |

### 定义基底

| 定义 | 版本 | 维度 |
|------|------|------|
| K线 | — | 基础 |
| 包含关系 baohan | v1.3 | 单标的 |
| 分型 fenxing | v1.0 | 单标的 |
| 笔 bi | v1.4 | 单标的 |
| 线段 xianduan | v1.3 | 单标的 |
| 中枢 zhongshu | v1.3 | 单标的 |
| 走势类型 zoushi | v1.6 | 单标的 |
| 背驰 beichi | v1.1 | 单标的 |
| 级别递归 level_recursion | v1.0 | 单标的 |
| 买卖点 maimai | v1.0 | 单标的 |
| 区间套 | — | 单标的 |
| 比价关系 bijia | v1.0 | 跨标的 |
| 等价关系 dengjia | v1.1 | 跨标的 |
| 流转关系 liuzhuan | v1.0 | 跨标的 |

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
├── definitions/           #   14 个核心定义（只通过仪式修改）
├── genealogy/             #   谱系记录
│   ├── settled/           #     100 条已结算
│   ├── pending/           #     生成态矛盾
│   └── dag.yaml           #     谱系 DAG（100 节点 / 445 边）
├── dispatch-dag.yaml      #   蜂群拓扑 v3.1（9 业务节点 + genome_layer + platform_layer）
├── pattern-buffer.yaml    #   知识结晶候选
└── sessions/              #   会话状态快照（热启动用）

.claude/                   # 递归拓扑异步自指蜂群基础设施
├── agents/                #   19 个 agent 定义
│   ├── genealogist.md     #     谱系工位（dominator）
│   ├── quality-guard.md   #     质量守卫（dominator）
│   ├── meta-observer.md   #     元观察者（dominator）
│   ├── gemini-challenger.md #   异质否定源
│   └── ...
├── hooks/                 #   21 个 hook（系统免疫层）
├── commands/              #   /ceremony, /inquire, /escalate, /ritual
├── skills/                #   结晶的知识能力
└── settings.json          #   hook 注册 + 权限配置

docs/
├── chanlun/text/          # 缠论原文（三级权威链）
│   ├── blog/              #   缠师原始博文（一级权威）
│   ├── chan99/            #   编纂版（二级权威）
│   └── mindmaps/         #   思维导图（三级）
└── spec/                  # 10 份规则规范

tests/                     # 100 个测试文件
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
