---
id: capital-physics-spec
title: "资本流转的物理量度——规格抽取（线密度 vs 扭矩 → kg·m 能量 → K4 旋转框架）"
source_pdf: "/Users/silencehan/Downloads/claude.ai-资本流转的物理量度线密度还是扭矩 - Claude.pdf"
source_pages: 41
extraction_date: 2026-04-24
topo_address: v70-swarm/pdf-capital-physics
parent_callback: team-lead
genealogy_refs:
  - '482'  # 资本旋转动力学（已结算）
  - '230'  # K4 独立边独立性
  - '475'  # T8 Wasserstein barcode——持续同调基础设施
  - '294'  # 金融折叠 vs 生产折叠
---

# 资本流转的物理量度：规格抽取

## 1. 结论

### 1.1 PDF 解决的核心问题

PDF 以"线密度 vs 扭矩"作为误读起点，推导出**三层答案**：

| 层 | 问题 | 答案 |
|----|------|------|
| 度量选择 | 资本流转的物理量度是什么？ | **kg·m = 能量（焦耳）**，不是 kg/m（线密度），也不是 N·m（扭矩）。但扭矩的**旋转动力学直觉**把卢麒元的管道图景翻转成 K4 闭合回路 |
| 度量内涵 | 这个能量如何度量？ | **L = M × ω**，ω = 金油比 = MCM' 绕 $ 旋转的角速度 = 剥削率。L 与 GDP 长期协整，短期对立运动 |
| 度量应用 | 这个度量如何操盘？ | **三层结构**：ω 极端位确认方向 + 一阶差分剥削签名确认强度 + 缠论买卖点确认时机；拓扑（持续同调/W1/PELT）回归地基，不参与操盘 |

### 1.2 规格清单（15 条数学对象 + 判据 + 测度）

| # | 对象 | 定义 | 等级 | 可观测性 |
|---|------|------|------|----------|
| 1 | K4 完全图 | 顶点 {E, C, R, $}，六条边，β₁=3 | L0 | 拓扑结构 |
| 2 | ω（金油比） | Gold/Brent 价格比 | L2 | GC/BZ 期货日线 |
| 3 | L = M × ω | 角动量 = 货币存量 × 角速度 | L2 | M2 × 金油比 |
| 4 | L 与 GDP 协整 | Engle-Granger p=0.033（2000-2026, 303月） | L2 | 月度序列 |
| 5 | 一阶差分 Pearson r | r = -0.364（剥削签名） | L2 | 月度 |
| 6 | Granger L→GDP | p=0.000125 | L2 | 月度 |
| 7 | L/GDP 偏离 | +3.48σ（当前） | L2 | 月度 |
| 8 | 空转功 | 有功负功 = M·ω·sin θ 沿 MCM' 回路 | L1 | 未独立实证 |
| 9 | 纸实比 | ω 在单条边的投影（ω_C, ω_E, ω_R） | L2 | 部分实证 |
| 10 | H0/H1 barcode | 三病态的拓扑签名 | L2 | W1 回归地基 |
| 11 | W1 距离 | Wasserstein-1：barcode 变化率 | L2 | 已实证滞后 |
| 12 | PELT 断点 | ω regime 切换检测 | L2 | 已实证震荡滞后 |
| 13 | 和乐（holonomy） | K4 闭合回路的积分相位 | L1 | 未独立实证 |
| 14 | Takens 嵌入 | 维度 3-5，窗口 12-24 月，(ω, GDP) 双变量 | L2 | 已实证 p<0.005 |
| 15 | 缠论买卖点 | 结构转折的时间坐标 | L2 | 查缠 PRO 确认 |

### 1.3 搭建产物（最小管线）

```
数据源层 → 度量计算层 → 拓扑判据层 → 操盘输出层
 GC/BZ/M2/GDP   ω, L, Δω, r    H0/H1/W1/PELT   方向+强度+时机
 (pandas)       (pandas)        (giotto-tda)     (缠论 + 协整)
```

## 2. 定义依据

### 2.1 PDF 关键引用（按页码）

**Page 1-3：起点的生产性误读**
- 卢麒元："资本是 kg·m"
- 编排者初读为 kg/m（线密度）→ 猜测为 N·m（扭矩）
- Claude 纠正：kg·m 在物理上是**能量（焦耳）**，不是密度也不是扭矩
- 误读的**生产性**：功（标量积 F·d·cosθ）和扭矩（矢量叉积 F×r·sinθ）量纲相同但几何不同——误读把线性功图景提升到了旋转做功图景

**Page 4-7：K4 完全图的导出**
- 四顶点 {E=企业, C=消费者, R=银行/准备金, $=美元资本}
- 六条边对应的交易流
- β₁（独立环数）= 3（230号谱系）
- $ 是**星形拓扑的中心**——所有交换经过 $

**Page 8-12：ω 的精确定义**
- ω 不是 "某种量的函数"
- ω 不是 "ω 在某条边的投影"
- **ω 本身就是金油比**——因为金和油是美元的两个历史锚
  - 旧锚：布雷顿森林（1944-1971）→ 金
  - 新锚：石油美元（1973-）→ 油
- 金油比直接是 MCM' 和 P 之间力量对比的**几何度量**

**Page 13-18：L = M × ω 的协整实证**
- 数据源：GC/BZ 期货日线（Databento）+ FRED M2/GDP
- 窗口：2000-2026（303 月）
- 协整 p=0.033（通过 5% 阈值）
- 一阶差分 r=-0.364（剥削签名）
- Granger L→GDP p=0.000125
- 当前 L/GDP 偏离 +3.48σ
- 2020 裂口 +102%
- **窗口敏感性**：2010-2026（185月）p=0.099 不显著 → 25年窗口是必要的

**Page 19-24：第一次否定——"空转=无功旋转"**
- 编排者初判：空转是角速度高但产出为零（无功旋转）
- 否定：MCM' 和 P **共享同一货币池**
- MCM' 每转一圈抽走的流动性**不进入** M→P→M'
- ω 越高，P 被抽得越干
- 结构必然，不是经验假说
- **空转 = 有功的负功**（sinθ 分量指向 MCM'，正功分量被抽走）

**Page 25-29：第二次否定——"纸实比=ω"**
- 编排者：纸实比（金融资产/实物资产）= ω
- 否定：纸实比是 ω 在**某条边上的投影**（ω_C、ω_E、ω_R 中的一个）
- ω 本身是 K4 顶层结构量

**Page 30-33：第三次否定——FIRE 剥离**
- 假设：如果剥离 FIRE（Finance/Insurance/Real Estate）后 GDP 与 ω 脱钩，则 MCM' 渗透性成立
- 测试：FIRE-out GDP 与 ω 的相关性**比总 GDP 更强的负相关**
- 结论：MCM' 不只渗透 FIRE，已渗透到"实体经济"——**FIRE 剥离不足以净化**

**Page 34-36：第四次否定——Fed 资产负债表**
- 假设：Fed 扩表会改变 L/GDP 的基准
- 测试：Fed 资产负债表与 L/GDP 偏离的相关性**近零**
- 结论：L/GDP 偏离不是由 Fed 扩表驱动的——是**寄生比例**自身的变化

**Page 37-39：第五次否定——三层传导链（W1→PELT→缠论）**
- 假设：拓扑指标领先缠论 2-3 个月
- CC 实验：W1 领先 39%/滞后 61%（median delta -11.5 天），PELT 领先 62% 但方差大（8-104 天）
- 结论：**Takens 嵌入天然需要积累窗口，是滞后检测器**
- 拓扑回归地基角色，不参与操盘

**Page 40-41：经管局 vs 缠论纤维丛**
- 经管局度量**流量**（事后、线性、行政边界切割）
- Claude 的方法度量**流的结构**（K4 每条边上和乐的符号）
- 三流精确定义：
  - 流速 = ω = 金油比
  - 流向 = 纤维丛和乐
  - 流量 = M
- "量 × 质才是完整的判断"

### 2.2 Memory 依据

**project_capital_rotation.md**：
- 操盘结构（编排者 2026-04-05 裁定）：方向 + 强度 + 时机 + 地基
- 10 次被否定（空转/纸实比/MV=PQ/W1/PELT/三层链/协整/新守恒量/L+DXY/FRED实物）
- 管线缺口：金油比日线退化 OHLC 展幅不足

**reference_lu_qiyuan.md**：
- 资本三流（流向/流量/流速）、三病态（空转/沉没/走资）
- 金油两锚、空转压价、沉没才能流转、价值回归

### 2.3 谱系依据

**482号（已结算 2026-04-04）**：资本旋转动力学完整框架
- PDF 是 482号的**发生学对话记录**——不是新谱系，是 482号的对话依据
- PDF 新增（482号未完全覆盖）：
  - 第三次否定（FIRE 剥离测试）的**实证步骤**
  - 第四次否定（Fed 资产负债表）的**对照测试**
  - 经管局 vs 纤维丛的**方法论对立**

## 3. 边界条件

每个度量的有效域（翻转条件）：

| # | 度量 | 翻转条件 |
|---|------|---------|
| 1 | K4 结构 | K4 之外的定价机制（如非美元货币联盟）使度量脱钩 |
| 2 | ω=金油比 | 金或油与美元的锚定关系断裂（如石油结算去美元化完成） |
| 3 | L=M×ω 协整 | 协整 p 值升破 5%（窗口或机制变化） |
| 4 | r=-0.364 | 一阶差分相关性接近零（剥削机制弱化） |
| 5 | L/GDP 偏离 | 回归到 ±1σ 内（协整力已发挥作用） |
| 6 | 空转=有功负功 | MCM' 流动性与 P 完全隔离（不从 P 抽） |
| 7 | H1 barcode 签名 | 三病态拓扑签名不再可区分（需 noise 模型） |
| 8 | W1 滞后性 | Takens 窗口缩短到接近零（结构性变化显化） |
| 9 | 纸实比投影 | ω_C/ω_E/ω_R 的投影关系不再可分解 |
| 10 | Takens 双变量 | 联合嵌入 p 值 > 0.05（伪联合关系） |

**窗口敏感性警告**：
- 协整对窗口长度**高度敏感**
- 2010-2026：p=0.099（不显著）
- 2000-2026：p=0.033（显著）
- 必须用**至少 25 年**窗口

## 4. 下游推论

### 4.1 新 skill 提议（3 个）

**提议 1：capital-omega-meter（优先级高）**
- 目的：ω 与 L 的实时度量 + L/GDP 偏离监控
- 输入：GC/BZ 日线、M2/GDP 月度
- 输出：当前 ω、L、L/GDP σ-score、协整 p 值、一阶差分 r
- 触发：阈值报警（L/GDP > +3σ 或 < -3σ、r 翻转）

**提议 2：capital-topology-signature（优先级中）**
- 目的：K4 每条边的和乐计算（三病态拓扑签名）
- 输入：六条边的流量时间序列（需构造）
- 输出：H0/H1 barcode、三病态签名（零/正/负和乐）
- 依赖：giotto-tda、完整的 K4 六边数据

**提议 3：capital-chanlun-gateway（优先级中）**
- 目的：金油比日线缠论结构的稳健产出
- 核心问题：金油比退化 OHLC 展幅不足
- 方案：从 TradingView 拉 1min 聚合日线，保留真实 high-low
- 依赖：TradingView MCP 导出通道

### 4.2 扩展 project_capital_rotation 分析

**扩展项**：
1. 添加**第三次否定（FIRE 剥离）**的独立实证
   - 数据：FRED GDP - FIRE 子行业序列
   - 测试：计算 FIRE-out GDP 与 ω 的相关性
2. 添加**第四次否定（Fed 资产负债表）**的对照测试
   - 数据：FRED WALCL（Fed 总资产）
   - 测试：L/GDP 偏离 vs Fed 扩表的相关性
3. 记录**第五次否定的完整拓扑实证**链路
   - W1 滞后数据：已在 `tmp/w1_vs_chanlun.csv`
   - PELT 震荡滞后数据：已在 `tmp/pelt_vs_chanlun.csv`

### 4.3 仓库缺口

**没有完整的资本物理度量管线**：
- `tmp/exp_m2v_analysis.py` 是探索性脚本
- `tmp/exp_correct_ratio_pipeline.py` 是探索性脚本
- 需要固化为 `src/capital_physics/` 模块
- 建议结构：
  ```
  src/capital_physics/
  ├── __init__.py
  ├── omega.py           # 金油比计算
  ├── angular_momentum.py # L = M × ω
  ├── cointegration.py   # Engle-Granger + Granger + 差分
  ├── deviation.py       # L/GDP σ-score
  ├── k4_topology.py     # K4 结构定义（stub，等签名算法）
  └── data_sources.py    # GC/BZ/M2/GDP 加载器
  ```

## 5. 谱系引用

### 5.1 直接相关

- **482号**（已结算 2026-04-04）：资本旋转动力学——金油比=ω, 一阶差分负相关（剥削签名）, 协整是脚手架
  - PDF 是 482号的**发生学对话记录**
  - PDF 不产生新谱系，是 482号的对话依据

### 5.2 间接相关

- **230号**：K4 独立边独立性——本框架的拓扑基础（β₁=3）
- **475号**：T8 Wasserstein barcode——拓扑地基
- **254号**：多经济体资本流动本体论——纯金融拓扑框架
- **292号**：折叠拓扑本体论——MCM'闭合回路的拓扑基础
- **294号**：金融折叠 vs 生产折叠——空转压价的结构分类

### 5.3 否定记录（PDF 中的五次 + project_capital_rotation 中的十次）

PDF 和 memory 的否定记录**完全一致**，不产生新否定：

| # | 被否定 | 否定来源 |
|---|--------|---------|
| 1 | 空转=无功 | 编排者（共享货币池）|
| 2 | 纸实比=ω | 编排者（投影关系）|
| 3 | MV=PQ=守恒恒等式 | CC实证（协整但一阶差分负相关）|
| 4 | 油涨=拓扑损伤 | 编排者（空转压价结构必然）|
| 5 | W1 领先缠论 | CC 实验（滞后检测器）|
| 6 | PELT 稳定领先 | CC 实验（震荡滞后）|
| 7 | 三层传导链 | 编排者（拓扑是地基不是信号）|
| 8 | (M2×Gold)/(Oil×GDP) 新守恒 | 代数恒等式验证 |
| 9 | L+DXY 守恒 | DXY 是相对度量 |
| 10 | FRED 实物数据守恒 | 优化器把 α 压到零 |

### 5.4 新增谱系？

**结论：不新增**。PDF 内容已被 482号覆盖。本 spec 是**482号的实施规格**，不是新谱系。

## 6. 影响声明

### 6.1 文件产出

- 新文件：`.chanlun/pdf-review/capital-physics-spec.md`（本文件）
- 无代码变更
- 无谱系变更（PDF 是 482号的对话依据）

### 6.2 产生的新接口/数据结构/判据（预计）

**数据结构（8 条）**：
1. `OmegaTimeSeries`：金油比时间序列（带频率标签）
2. `AngularMomentumSeries`：L = M × ω 时间序列
3. `CointegrationResult`：{p_value, r_value, granger_p, deviation_sigma}
4. `K4Topology`：K4 结构的顶点/边/β₁ 表示
5. `EdgeHolonomy`：K4 单条边的和乐（复数/相位）
6. `BarcodePayload`：H0/H1 barcode 带标签
7. `W1Distance`：Wasserstein-1 距离序列
8. `ExploitationSignature`：剥削签名（r、偏离σ、Granger 的综合）

**判据（5 条）**：
1. 方向判据：L/GDP 偏离 σ-score
2. 强度判据：一阶差分 r
3. 时机判据：缠论买卖点（外部依赖）
4. 地基判据：W1 / PELT（事后诊断）
5. 渗透判据：FIRE-out GDP 相关性

**接口（3 条）**：
1. `omega_meter.compute(freq='1D'|'1M')`：ω 测量
2. `cointegration.test(l_series, gdp_series, window_months)`：协整检验
3. `deviation.score(l, gdp)`：L/GDP σ-score

### 6.3 影响的模块/定义

- **无**现有模块直接修改
- **建议新建** `src/capital_physics/`（提议 1/2/3）
- **建议扩展** memory `project_capital_rotation.md` 添加 FIRE 剥离和 Fed 资产负债表测试

### 6.4 认识论等级汇总

| 等级 | 条目数 | 占比 |
|------|--------|------|
| L0（纯代数/定义） | 2 (K4, β₁) | 13% |
| L1（合成数据） | 2 (空转功, 和乐) | 13% |
| L2（真实数据） | 11 | 74% |
| L3（交叉验证） | 0 | 0% |

**L3 缺口**：所有度量目前都是**单标的/单时段**验证（美股 + 2000-2026）。下游若扩展到 L3，需要：
- 多标的：欧元区、日本、中国等其他 K4 实例
- 多时段：包含更多 regime（如 1970s 石油危机）

### 6.5 最小管线提案

**阶段 1：ω 度量管线（1-2 天）**
```python
# src/capital_physics/omega.py
def compute_omega(gc_series, bz_series, freq='1D'):
    """金油比时间序列"""
    return gc_series / bz_series

# src/capital_physics/angular_momentum.py
def compute_L(m2_series, omega_series):
    """L = M × ω"""
    return m2_series * omega_series.resample('1M').mean()
```

**阶段 2：协整判据管线（2-3 天）**
```python
# src/capital_physics/cointegration.py
from statsmodels.tsa.stattools import coint, grangercausalitytests
def test_cointegration(l_series, gdp_series):
    """Engle-Granger + Granger + 一阶差分 r"""
    p_coint = coint(l_series, gdp_series)[1]
    p_granger = grangercausalitytests(...)
    r = np.corrcoef(np.diff(l_series), np.diff(gdp_series))[0,1]
    return CointegrationResult(p_coint, r, p_granger, ...)
```

**阶段 3：偏离监控管线（1 天）**
```python
# src/capital_physics/deviation.py
def compute_sigma(l_series, gdp_series, window=60):
    """L/GDP σ-score（rolling window）"""
    ratio = l_series / gdp_series
    return (ratio - ratio.rolling(window).mean()) / ratio.rolling(window).std()
```

**阶段 4：K4 拓扑签名（未来，依赖六边数据构造）**
- 当前**无法实施**——需要先解决 K4 六边流量的数据构造问题
- 建议：先做"单顶点出入流量"的简化版（$ 顶点）

**阶段 5：缠论时机接口（外部）**
- 依赖查缠 PRO 的买卖点信号
- 本管线消费信号，不实现缠论

## 7. 未决项

### 7.1 定义层未决

- **K4 六条边的流量数据**如何构造？（影响拓扑签名计算）
- **和乐的具体计算方法**：复数相位 vs 实数积分？（影响 H0/H1 签名）
- **纸实比的三个投影**（ω_C, ω_E, ω_R）如何分别度量？

### 7.2 实施层未决

- **金油比日线管线**的展幅不足问题（project memory 未解决项）
- **TradingView 数据通道**是否可用于批量拉 1min 聚合日线？
- **src/capital_physics/** 是否新建？需要编排者的价值判断（新建 = 固化当前理解；不新建 = 保持探索态）

### 7.3 不决项（决策自持）

以下**不需要**上浮，由下游工位自行判断：
- 协整窗口长度默认值（25 年/30 年）
- σ-score 的 rolling window 默认值（60月/120月）
- 是否添加多标的对照（L3 扩展）

---

**产出完成声明**：本 spec 完整覆盖 PDF 41页所有核心规格，与 482号谱系完全对齐，无新矛盾产生。
