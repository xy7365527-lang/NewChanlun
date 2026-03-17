---
id: '477'
number: 477
title: "473/474/475号下游推论批量消化——22条四分法处置"
type: 消化记录
status: 已结算
date: 2026-03-17
source: "[蜂群] v260-swarm/downstream-consume 工位"
negation_source: ""
negation_form: ""
topo_effect: ""
depends_on:
  - '473'   # Morse 命名门槛
  - '474'   # 操作/选择/命名三层分离
  - '475'   # T8 Wasserstein 条形码重表述
epistemological_level: "L0（代码审查 + 定理推导）"
tensions_with: []
---

# 477号：473/474/475号下游推论批量消化

**认识论等级**: L0

## 概述

对 473号（3条）、474号（3条）、475号（16条）共 22 条下游推论进行四分法分类并逐条处置。

---

## 473号下游推论（3条）

### 推论1：non_critical 顶点在 fold 优先级中降权

**四分法分类**: 行动类（代码检查）

**检查结果**: traversal.py 中 fold 选择逻辑（L542-589）**无优先级机制**。当前 fold 检测有两条路径：
- Path A（L549-568）：f=0 categorical fold，遍历邻居检查 f 值，无优先级排序
- Path B（L570-589）：shared neighbor fold，遍历历史顶点检查共享邻居，无优先级排序

两条路径都是"先遇到先触发"的顺序，不区分 critical/non_critical。

**处置**: 标记为**待实装谱系**。当前 fold 选择逻辑不使用 non_critical 标记，如果未来需要在多个 fold 候选中择优，non_critical 可作为降权信号。但当前不存在"多候选竞争"场景（检测到第一个即返回），因此此推论在当前架构下无实装点。

**状态**: 已覆盖审查，无需代码变更。如果 fold 选择逻辑未来引入多候选排序，此推论激活。

### 推论2：critical 标记作为穿越轨迹分析信号

**四分法分类**: 行动类（代码检查）

**检查结果**: trajectory_cluster.py **未使用** critical 标记。grep 确认：该文件不包含 `critical`、`non_critical`、`terrain`、`morse` 任何关键词。

**处置**: 标记为**待实装**。TrajectoryCluster 当前不消费 critical 标记。如果未来需要分析穿越"关键路径"（critical 操作的分布模式），需在 trajectory_cluster.py 中引入 StepLog.critical 字段的消费逻辑。

**状态**: 已覆盖审查，无需当前代码变更。推论有效但尚无消费者。

### 推论3：daemon.py 日志标注使 CRITICAL/non-critical 运行时可观测

**四分法分类**: 行动类（代码检查）

**检查结果**: daemon.py L224-234 **已实装**。`format_step_log` 函数根据 `log.critical` 值输出 ` CRITICAL` 或 ` non-critical` 标注。

**处置**: **已覆盖**，无需变更。

**验证**: daemon.py:224-234 明确包含：
```python
if log.critical is True:
    crit_mark = " CRITICAL"
elif log.critical is False:
    crit_mark = " non-critical"
```

---

## 474号下游推论（3条）

### 推论1：操作层禁止加入"重要性"判断

**四分法分类**: 定理类（架构守护规则）

**检查结果**: engine.py 中 fold/negate/sublate 函数**不包含**任何"重要性"判断代码。grep 确认：engine.py 中无 `重要`、`importan`、`significance`、`relevant`、`weight.*vertex` 等关键词（唯一命中是 L1611 的注释"顺序至关重要"，指代码执行顺序，不是顶点重要性判断）。

engine.py 的 Vertex dataclass 有 `non_critical` 字段（L82），但此字段仅被**记录**（traversal.py 写入），engine.py 的操作函数（fold/negate/sublate）不读取此字段来做任何决策。

**处置**: **已遵守**。474号的架构守护规则在当前代码中未被违反。

### 推论2：命名层禁止修改遭遇检测逻辑

**四分法分类**: 定理类（架构守护规则）

**检查结果**: traversal.py 中命名层（D 判据，在 execute_encounter 中计算 beta_1 差值并标记 critical/non_critical）和选择层（detect_encounter 中的遭遇检测）是**分离的**：
- 命名层在 execute_encounter 的 SUBLATION/NEGATE_B 分支中（操作已完成后才计算 D）
- 选择层在 detect_encounter 中（terrain + f_criterion）
- 命名层不修改任何影响 detect_encounter 行为的变量

**处置**: **已遵守**。两层逻辑独立，命名层是纯事后标记。

### 推论3：三层分离是 370号架构表达

**四分法分类**: 定理类（从已结算谱系推导）

**推导**:
```
370号：Morse 有效域精确终止于关键性判断
  → 关键性判断 = 选择层（terrain 标记 critical edge）
  → 操作层（engine.py fold/negate/sublate）不做关键性判断
  → 命名层（D 判据）是事后标记，不预测关键性
  → 三层分离 = 370号边界在代码架构中的投影
```

**处置**: **已确认**。474号是370号的架构表达——定理类，无需额外操作。

---

## 475号下游推论（16条）

475号共 4 大类 16 条推论。根据 475号谱系文件的明确记载，这些推论全部需要**分钟级K线数据（1min 或 5min），时间跨度 >= 1年**。

### 数据可用性评估

| 数据源 | 分钟级支持 | 长跨度可用性 | 评估 |
|--------|-----------|-------------|------|
| akshare（东方财富） | 1min/5min/15min/30min/60min | 仅最近几个月（东方财富分钟线历史有限） | **不足** |
| Alpha Vantage | 1min/5min | 免费tier仅最近 1-2 个月全量 | **不足** |
| databento（本地缓存） | 1min | 仅 Brent crude 2020-2026（1个标的） | **不足**（单标的不满足多标的要求） |

**结论**: 分钟级 >= 1年跨度的多标的数据**当前不可得**（免费数据源限制）。

### 16条推论逐条处置

**推论1-4（T8 L2 验证路径）**: blocked——需要分钟级多标的长跨度数据
- 推论1：T8 passed 率 vs 随机基线统计检验
- 推论2：T8 passed=False 且 MACD agrees=True 案例分析
- 推论3：递归级别 >= 1 的背驰样本收集
- 推论4：管线端到端：K线→笔→线段→中枢→走势类型→背驰→T8

**推论5-8（W_1 gauge 稳定性验证）**: 部分可执行
- 推论5：W_1 在 wide/strict/new 模式切换下的保持率——**可自主执行**（已有 5只A股 x 3模式日线数据）
- 推论6：W_1 保持率 >= Tier 1（73%）判据
- 推论7：与 209号 bottleneck Tier 3 的对比
- 推论8：gauge 不变性确认/否定

**注意**: 推论5-8 虽然部分可执行，但属于 L2 验证，不在本工位的定理/行动分类范围内。标记为**可执行但非本工位职责**。

**推论9-12（三算子方案映射验证）**: L0 验证
- 推论9：M_delta 与 a_inclusion/a_fractal/a_stroke 接口映射
- 推论10：Z 与 a_segment_v1/a_zhongshu_v1 接口映射
- 推论11：S 与 a_move_v1 接口映射
- 推论12：递归控制与 a_level_fsm 映射

475号谱系已明确声明这些是"L0（纯定义/映射）"且"不是新的代码变更需求"。**已在 475号中覆盖**，无需额外操作。

**推论13-16（条形码多维性）**: L0 概念分析
- 推论13：最长 bar 作为主中枢特征提取
- 推论14：bar 数量作为中枢个数指标
- 推论15：bar 分布作为中枢结构描述
- 推论16：与 MACD 三维度的互补性（210号）

475号谱系已作为概念分析覆盖。**已在 475号中覆盖**，无需额外操作。

### 475号总结

| 推论类别 | 条数 | 状态 | 阻塞原因 |
|---------|------|------|---------|
| T8 L2 验证路径 | 4 | **blocked** | 分钟级多标的长跨度数据不可得 |
| W_1 gauge 稳定性 | 4 | **可执行/非本工位** | 需独立工位执行 L2 验证 |
| 三算子映射 | 4 | **已覆盖** | 475号已声明为 L0 映射 |
| 条形码多维性 | 4 | **已覆盖** | 475号已作为概念分析覆盖 |

---

## 边界条件

| 条件 | 翻转阈值 |
|------|---------|
| 473推论1（fold降权） | 如果 fold 选择引入多候选排序机制，此推论激活 |
| 473推论2（trajectory critical） | 如果 trajectory_cluster.py 引入 critical 消费逻辑，此推论激活 |
| 475推论1-4（T8 L2） | 如果获得分钟级多标的长跨度数据源（如 databento 订阅扩展），阻塞解除 |
| 475推论5-8（W_1 gauge） | 需独立工位执行 |

## 影响声明

- **无代码变更**：22条推论全部为审查/分类/标记，无需修改现有代码
- **新增谱系**：477号（本文件）记录消化结果
- **确认 3 条已覆盖**（473推论3 + 474推论1/2/3）
- **确认 8 条已在 475号覆盖**（推论9-16）
- **标记 2 条待实装**（473推论1/2——当前无实装点）
- **标记 4 条 blocked**（475推论1-4——数据不可得）
- **标记 4 条可执行但非本工位**（475推论5-8——L2 验证）
