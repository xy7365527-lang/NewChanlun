---
id: '529'
number: 529
title: "K4 折叠通道重构——顶点标签模型→折叠通道模型（528号下游实装）"
type: 实现
status: 已结算
四分法分类: 行动（执行编排者重构指令）
date: 2026-06-07
settled_date: 2026-06-07
source: "编排者重构指令 + docs/architecture/k4_reconstruction_research.md §8 + 528号下游动作"
depends_on:
  - '528'   # 顶点映射折叠压平冲突（本号是其 §8.5 标注的下游实装）
  - '292'   # 折叠拓扑本体论——折叠通道定义
  - '254'   # 黄金∈Σ∩C、油=结算通道、K4→K3 退化
  - '527'   # σ=走势方向、81 边图（本号实装 transition.py 81 边）
  - '330'   # K4 资本循环映射——顶点资本形态 M/P/C/R 命名根据
related:
  - '259'   # C 顶点代理需结算属性（折叠通道模型下重新分工）
  - '026'   # cash 双重身份——M 既是顶点又是度量尺
epistemological_level: "L0（结构重构，代码遵循已结算谱系；无数据验证，未声明任何 L2 经验结论）"
topo_effect: "K4 顶点从单标的标签升级为折叠通道承载结构；金/油从硬编码顶点降为折叠通道观测量；transition.py 从 54 边（527号实现错误）改为 81 边；新增单一真相源 data_mapping.py 消除 528号四套冲突映射。"
---

## 结算结论（2026-06-07，四分法=行动）

本号是 528号 §8.5 标注的下游实装——把"折叠成为代码一等原语"从理论落地为代码。
不产生新概念（全部概念来自 292/254/527/330 已结算谱系），故四分法=行动（执行），
非定理也非选择。编排者已给出重构方向（折叠通道模型），蜂群执行。

## 实装内容

### 1. 顶点重命名 M/P/C/R（330号资本循环语义）
`graph.py`：旧 `Vertex` E/C/R/CASH → P/C/R/M。所指（标的）不变，仅名称对齐 330号
资本形态（$=货币资本 M、Equity=生产资本金融化 P、商品资本 C、不动产 R）。

### 2. 折叠通道成为一等结构（`fold_channel.py`，新建）
- `FoldChannel` dataclass：折叠通道是**观测量**，在 K4 边上读出，不是顶点不是边。
- `AU`：C↔M 双向折叠（黄金∈Σ∩C），观测于 C/M 独立边，W=4。
- `OIL`：C→P 定向通道（产业循环物质投入），观测于 P/C 派生边，W=3。
- `omega()`：ω=金/油（482号剥削率）。仅长期方向，不接实时门控（Omega Regime 已证伪）。

### 3. 配置空间（`config_space.py`）
Γ=(σ_P, σ_C, σ_R)；σ_M 不参与（M 是度量尺，026号双重身份）；折叠通道 ω 为附加信号，
不进主轴。

### 4. transition.py 改 81 边（527号定理实装）
删除 +↔− 禁令（54 边实现错误）；adjacent = 单分量改为任意方向；边数 81、直径 3、
非二部；距离改为 Hamming。旧 54 边"价格位置态投影"依 no-patch 删除（无真实消费者）。

### 5. CROSS_NATIONAL 实装（`cross_national.py`，新建）
254号定义2 多经济体共展开：国内边 6n + 货币边 n(n−1)/2 + 跨国同类边 3·n(n−1)/2，
独立自由度 4n−1。

### 6. 单一真相源（`data_mapping.py`，新建）
消除 528号四套冲突映射。M→UUP、P→ES（可用）；C→DBC、R→TLT（1min 缺口，诚实标注
NEEDS_ACQUISITION/DAILY_ONLY，**不伪造替代**，231号）；Au→GC、Oil→CL（折叠通道观测量）。

## 诚实标注的张力（不静默覆盖）

### 张力1：Oil = C→P（本号）vs Oil = C↔$（292号折叠盘点）
292号§3.1 列 Oil 折叠为 C↔$（石油美元）；§5 列油的扩张面=燃烧（C→P 物质消耗）、
收缩面=石油美元（C→$）。本号按编排者指令 + 330号产业循环视角实装 C→P **扩张相位**，
292号的 C↔$ **石油美元相位**为已知未建模边界——`FoldChannel` 结构支持未来扩展，
不声明已建（避免声明膨胀，090号）。两相位可调和（同一对象不同相位，292号折叠定义），
不构成矛盾，不满足 escalate 条件。

### 张力2：消费者层绑定偏离（528号下游待办）
`k4_integration.py` 的 M2 绑定（σ_c=金、σ_r=油）与正典折叠通道映射不一致——把折叠
通道观测量塞进配置主轴。本号统一**正典层**（graph/config_space/data_mapping），但未
重写 M2 业务绑定（会破坏 M2 回测），属 528号"复核回测↔实时可比性"下游，已在代码
docstring 诚实标注，不静默。

## 与 528号争议的关系

528号"定理/吸收"分类有未决质询（裂隙3：截面问题——回测美元截面 vs 监控黄金截面）。
本号**不重开 528号分类**（那是 Lead/编排者的裁定权，no-unnecessary-escalation）。
折叠通道模型恰好解决裂隙3：金作为 C↔M 折叠通道，其相位由所选结算尺截面决定——
美元截面读 C 相位，黄金截面读 $ 相位（C=$ 退化），同一通道两个读数，不再是冲突。

## 边界条件（结论翻转条件）

- 若结算尺空间 Σ 演化（254号公理0），C 顶点代理可能变（259号自陈"随公理0演化"）。
- 若未来需建模 Oil 的 C↔$ 石油美元相位，扩展 OIL 折叠通道（结构已支持）。
- 若 C/R 的 1min 数据源到位，data_mapping 缺口标注更新为 AVAILABLE。
- 81 边图性质（直径3/非二部）依赖 527号 σ=走势方向定论；若 527号被编排者 INTERRUPT 翻转，transition.py 回退。

## 影响声明

- 改动：`topology/{graph,config_space,transition}.py` 重构；新建 `topology/{fold_channel,
  cross_national,data_mapping}.py`；`fiber_bundle.py`（beta_er→beta_pr）；消费者层
  sigma_e→sigma_p 改名（k4_scanner/fiber_pipeline_adapter/full_pipeline/stock_scanner/
  scanner_pool/k4_integration + scripts）。
- 测试：test_graph/test_config_space/test_transition 更新为新模型；新建 test_fold_channel/
  test_cross_national/test_data_mapping。全部 topology 测试通过（4555 passed，81 失败为
  预存的 pydantic/genai/server 依赖缺失 + 数据文件缺失，已用 git stash 确证零耦合）。
- 未改：matrix_topology.py（独立 AssetVertex 枚举，不在本次目标）；k4_integration M2 业务
  绑定（528号下游）。
