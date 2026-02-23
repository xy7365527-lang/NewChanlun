# 跨标的3条定义审计报告

**审计工位**: audit-cross
**审计时间**: 2026-02-23
**审计范围**: 比价关系(bijia)、等价关系(dengjia)、流转关系(liuzhuan)

---

## 1. 比价关系（bijia, #12）

### 定义摘要

- 两个标的之间的价格比值关系，其变动构成独立买卖系统
- 比价K线 = A/B 逐字段除法（OHLC），volume 取 A
- 比价K线直接输入标准管线（包含处理→分型→笔→线段→中枢→走势类型）
- 不变量：IR-1（对称性）、IR-2（独立性）、IR-3（完备性）

### 实现文件

| 模块 | 路径 | 职责 |
|------|------|------|
| equivalence.py | `src/newchan/equivalence.py:199-216` | `make_ratio_kline()` — 比价K线构造 |
| synthetic.py | `src/newchan/synthetic.py:40-58` | `make_ratio()` — 历史接口，委托给 equivalence |
| ratio_engine.py | `src/newchan/ratio_engine.py:47-55` | `_run_pipeline()` — 比价K线→标准管线 |
| ratio_engine.py | `src/newchan/ratio_engine.py:58-93` | `analyze_pair()` — 单对分析调度 |
| capital_flow.py | `src/newchan/capital_flow.py:78-143` | `strokes_to_flows()` — 比价笔→资本流转语义 |
| cli.py | `src/newchan/cli.py:231-260` | `_cmd_synthetic --op ratio` — CLI 入口 |

### 定义-实现一致性

| 定义条目 | 实现状态 | 详情 |
|----------|---------|------|
| 比价K线 OHLC 除法 | **一致** | `make_ratio_kline()` 对 open/high/low/close 分别除法 |
| volume 取 A | **一致** | `result["volume"] = a["volume"]`（第214行） |
| 自动时间对齐 | **一致** | inner join on index（第207行） |
| B价格为0过滤 | **一致** | `validate_pair()` → `_align_pair()` 第74行检查 |
| 比价K线送入标准管线 | **一致** | `ratio_engine._run_pipeline()` 调用 merge_inclusion→fractals→strokes→segments |
| IR-1 对称性 | **一致** | 测试验证 A/B 上涨 ⟺ B/A 下跌 |
| IR-2 独立性 | **一致** | 测试验证 A涨+B涨更快→比价跌 |
| IR-3 完备性 | **一致** | E2E 测试验证比价K线产出分型/笔/线段 |
| 比价走势携带资本流转语义 | **一致** | `capital_flow.py` 完整实现语义映射表 |

### 语义偏差

无。

### 未声明行为

无。

### 缺口

| 缺口 | 严重性 | 说明 |
|------|--------|------|
| 中枢/走势类型/背驰 未在比价管线中显式集成 | LOW | `ratio_engine._run_pipeline()` 止步于线段。定义声明"携带资本流转语义"包括"比价背驰"和"比价走势完成"，但代码中没有从线段→中枢→走势类型→背驰的管线。单标的管线中这些功能也尚未完全实现（属于全局进度问题而非比价特有缺口） |

### 测试覆盖

| 测试文件 | 用例数 | 状态 |
|----------|--------|------|
| test_equivalence.py | 16 | 全通过 |
| test_ratio_engine.py | 15 | 全通过 |
| test_ratio_pipeline.py | 4 | 全通过 |
| test_capital_flow.py | 18 | 全通过 |
| **小计** | **53** | **全通过** |

---

## 2. 等价关系（dengjia, #13）

### 定义摘要

- 等价对条件：可比性（重叠时间窗口）、非退化（比价非常数）、流动性
- 数学性质：满足对称性，不满足自反和传递（非严格等价关系）
- 025号谱系降格：从本体关系降为筛选条件
- 三层退化连锁检测（024号谱系）：CV预筛→结构退化→动力退化

### 实现文件

| 模块 | 路径 | 职责 |
|------|------|------|
| equivalence.py | `src/newchan/equivalence.py:18-34` | `EquivalencePair` 数据结构 |
| equivalence.py | `src/newchan/equivalence.py:37-53` | `ValidationResult` 诊断结果 |
| equivalence.py | `src/newchan/equivalence.py:70-98` | `_align_pair()` + `_layer1_cv()` — 对齐与CV预筛 |
| equivalence.py | `src/newchan/equivalence.py:101-154` | `_layer2_and_3()` + `validate_pair()` — 三层检测 |
| matrix_topology.py | `src/newchan/matrix_topology.py:41-48` | `MatrixEdge` — 四矩阵边=等价对 |

### 定义-实现一致性

| 定义条目 | 实现状态 | 详情 |
|----------|---------|------|
| C-1 可比性（重叠时间窗口） | **一致** | `_align_pair()` 检查 overlap ≥ MIN_OVERLAP(5) |
| C-2 非退化（比价非常数） | **一致** | 三层检测：CV预筛 + 结构退化 + 动力退化 |
| C-3 流动性 | **部分一致** | 定义声明"A和B都有充足的市场深度"，但代码中无显式流动性检查（volume阈值）。三层检测通过 MACD 动力退化间接覆盖了部分流动性问题（低流动性→低波动→Layer 1/2 拒绝），但不是直接的流动性验证 |
| EquivalencePair frozen | **一致** | `@dataclass(frozen=True, slots=True)` |
| label 属性 | **一致** | `f"{self.sym_a}/{self.sym_b}"` |
| 降格为筛选条件 | **一致** | `validate_pair()` 在 `ratio_engine.analyze_pair()` 中作为前置筛选调用 |

### 语义偏差

| 偏差 | 严重性 | 说明 |
|------|--------|------|
| C-3 流动性条件无显式实现 | MEDIUM | 定义文件明确列出流动性为三条件之一，但代码中没有 volume 或 bid-ask spread 相关检查。三层退化检测可间接过滤部分低流动性标的，但并不等价于流动性验证。定义文件的"当前实现"小节也未提及此缺口 |

### 未声明行为

| 行为 | 说明 |
|------|------|
| 三层退化连锁检测 | 定义文件未详述 Layer 2（结构退化/stroke intensity）和 Layer 3（动力退化/MACD dynamics）的具体阈值和算法。这些细节在代码中有，但定义文件只提"非退化检查（比价标准差 < degeneracy_threshold → 常数退化）"，这只对应 Layer 1 |

### 缺口

| 缺口 | 严重性 | 说明 |
|------|--------|------|
| 流动性条件缺失 | MEDIUM | 见上。需要决断：是补充代码还是从定义中降级此条件 |
| 定义文件中对三层检测的描述不完整 | LOW | 代码实际有三层，定义只描述了 Layer 1。建议定义文件补充 Layer 2/3 描述 |

### 测试覆盖

| 测试文件 | 用例数 | 状态 |
|----------|--------|------|
| test_equivalence.py | 16 | 全通过（含三层退化连锁检测） |
| test_matrix_topology.py | 28 | 全通过 |
| **小计** | **44** | **全通过** |

---

## 3. 流转关系（liuzhuan, #14）

### 定义摘要

- 四矩阵有向流场：4个资本容器顶点（动产/不动产/商品/现金）构成 K₄ 完全图 6条边
- 顶点流量：net(V) = Σ flow(eᵢ, V)
- 共振判定：|net(V)| ≥ 2 → 共振（强/弱）
- 守恒约束：Σ net(V) = 0
- 结构性同步（非时钟同步）
- 流转关系：有向、一对多（源→汇）

### 实现文件

| 模块 | 路径 | 职责 |
|------|------|------|
| matrix_topology.py | `src/newchan/matrix_topology.py` | AssetVertex(4顶点) + MatrixEdge(6边) + FourMatrix 拓扑容器 |
| capital_flow.py | `src/newchan/capital_flow.py` | FlowDirection 枚举 + StrokeFlow + strokes_to_flows() |
| flow_relation.py | `src/newchan/flow_relation.py:32-43` | EdgeFlowInput — 边方向输入 |
| flow_relation.py | `src/newchan/flow_relation.py:50-75` | ResonanceStrength + VertexFlowState — 共振强度 + 顶点状态 |
| flow_relation.py | `src/newchan/flow_relation.py:82-167` | _flow_contribution + aggregate_vertex_flows + detect_resonance |
| flow_relation.py | `src/newchan/flow_relation.py:181-268` | CashSignalAnalysis + disambiguate_cash_signal — 026号谱系现金边消歧 |
| flow_timeline.py | `src/newchan/flow_timeline.py` | 流转状态时间序列（结构同步而非时钟同步） |

### 定义-实现一致性

| 定义条目 | 实现状态 | 详情 |
|----------|---------|------|
| 4 个顶点 (EQUITY/REAL_ESTATE/COMMODITY/CASH) | **一致** | `AssetVertex` 枚举正确包含 4 个值 |
| K₄ 完全图 6 条边 | **一致** | `ALL_EDGES = tuple(combinations(AssetVertex, 2))`，验证 C(4,2)=6 |
| flow(e, V) = +1/-1/0 | **一致** | `_flow_contribution()` 正确实现方向到贡献值的映射 |
| net(V) = Σflow(eᵢ, V) | **一致** | `aggregate_vertex_flows()` 第147行 |
| net(V) ∈ {-3..+3} | **一致** | 测试验证范围 |
| \|net\| ≥ 2 → 共振 | **一致** | `_classify_resonance()` 正确实现阈值判定 |
| \|net\| = 3 → 强共振 | **一致** | 测试验证 |
| \|net\| = 2 → 弱共振 | **一致** | 测试验证 |
| Σnet(V) = 0 守恒约束 | **隐式一致** | 守恒是拓扑不变量（有向图的数学性质），代码没有显式 `check_conservation()` 函数但测试中验证了守恒成立 |
| 结构性同步（非时钟同步） | **一致** | `flow_timeline.py` 实现事件驱动的状态更新 |
| 现金边信号消歧（026号谱系） | **一致** | `disambiguate_cash_signal()` — 定义文件中未提及但作为扩展实现 |

### 语义偏差

无。

### 未声明行为

| 行为 | 说明 |
|------|------|
| CashSignalAnalysis | `flow_relation.py` 包含026号谱系的现金边信号消歧逻辑，这超出了 liuzhuan.md 定义的范围，但属于合理扩展。有独立测试覆盖(17用例) |
| FlowTimeline | `flow_timeline.py` 实现了流转状态时间序列，这是定义中"结构性同步"的具体实现。有独立测试覆盖(17用例) |

### 缺口

| 缺口 | 严重性 | 说明 |
|------|--------|------|
| `check_conservation()` 函数未实现 | LOW | 定义文件"待实现"列出 P2 优先级。守恒是数学必然（拓扑不变量），代码中 aggregate_vertex_flows 的输出天然满足守恒，但没有显式检查函数。E2E 测试中有守恒断言。实际影响低——如果未来需要检测"守恒破缺"信号（跨区域资本流动），需要此函数 |
| `VertexFlowState` 无 "源/汇/中性" 显式分类 | LOW | 定义文件"待实现"提到 `VertexFlowState` 应有源/汇/中性分类。当前实现通过 net_flow 正负和 strength 间接表达，但无 `FlowRole` 枚举（如 SOURCE/SINK/NEUTRAL）。功能等价，但接口不如定义所述清晰 |
| Flow(V → {W₁, W₂, ...}) 流转关系数据结构未实现 | MEDIUM | 定义描述了"有向、一对多的流转关系"作为最终输出，但代码止步于 VertexFlowState（每个顶点的 net_flow），没有从 VertexFlowState 聚合出 Flow(源→汇集合) 的数据结构 |
| 规范文件 flow_relation_v1.md 待建 | LOW | 定义文件声明 "规范文件：待建（flow_relation_v1.md）" |

### 测试覆盖

| 测试文件 | 用例数 | 状态 |
|----------|--------|------|
| test_flow_relation.py | 14 | 全通过 |
| test_cash_disambiguation.py | 17 | 全通过 |
| test_flow_timeline.py | 17 | 全通过 |
| test_flow_relation_e2e.py | 16+ | 依赖缓存数据，有条件跳过 |
| test_flow_timeline_e2e.py | — | 依赖缓存数据 |
| **小计（确定可跑）** | **48** | **全通过** |

---

## 4. 跨定义关联审计

### 依赖链完整性

```
等价关系(dengjia) → 比价关系(bijia) → 流转关系(liuzhuan)
    筛选层            原子层              拓扑层
```

| 依赖关系 | 状态 | 详情 |
|----------|------|------|
| dengjia → bijia | **一致** | `validate_pair()` 在 `analyze_pair()` 中作为前置筛选 |
| bijia → liuzhuan | **一致** | 比价笔 → `strokes_to_flows()` → `EdgeFlowInput` → `aggregate_vertex_flows()` |
| 四矩阵拓扑贯通 | **一致** | MatrixEdge ↔ EquivalencePair ↔ EdgeFlowInput 形成完整链路 |

### 偏序结构

定义声明"等价关系建立在比价关系的偏序结构之上"。代码中的实际关系是：等价关系 **筛选** 比价对（C-1/C-2/C-3 条件），筛选通过后才构造比价K线。这是一个 **前置条件** 关系而非偏序关系。025号谱系已将等价关系降格为筛选条件，代码与降格后的定义一致。

### 封闭性保证

等价关系不满足自反和传递——代码和定义在这一点上一致。没有代码假设等价关系具有封闭性。

---

## 5. 总体测试统计

| 模块 | 测试文件 | 用例数 |
|------|---------|--------|
| equivalence | test_equivalence.py | 16 |
| ratio_engine | test_ratio_engine.py | 15 |
| ratio_pipeline | test_ratio_pipeline.py | 4 |
| capital_flow | test_capital_flow.py | 18 |
| matrix_topology | test_matrix_topology.py | 28 |
| flow_relation | test_flow_relation.py | 14 |
| cash_disambiguation | test_cash_disambiguation.py | 17 |
| flow_timeline | test_flow_timeline.py | 17 |
| **总计** | **8 个文件** | **129 用例，全通过** |

E2E 测试（依赖缓存数据）：
- test_ratio_pipeline.py — 4 用例
- test_flow_relation_e2e.py — 16+ 用例
- test_flow_timeline_e2e.py — 待确认

---

## 6. 第二阶段进展总体评估

### 已实现模块

| 模块 | 完成度 | 说明 |
|------|--------|------|
| 比价K线构造 | **100%** | make_ratio_kline 完整实现，含时间对齐、OHLCV 列 |
| 等价对验证 | **90%** | 三层退化检测完整，缺显式流动性检查 |
| 比价管线集成 | **80%** | 到线段为止完整，缺中枢/走势类型/背驰 |
| 资本流转语义 | **100%** | strokes_to_flows 完整映射 |
| 四矩阵拓扑 | **100%** | 4顶点、6边、不可变更新、工厂函数 |
| 顶点流量聚合 | **100%** | aggregate_vertex_flows 完整 |
| 共振检测 | **100%** | 强/弱/无共振三级判定 |
| 现金边消歧 | **100%** | 026号谱系完整实现 |
| 流转状态时间线 | **100%** | 事件驱动，结构同步 |
| 守恒约束检查 | **50%** | 隐式满足，无显式函数 |
| Flow(源→汇) 数据结构 | **0%** | 未实现 |
| 批量分析调度 | **100%** | analyze_batch 完整 |
| CLI 集成 | **100%** | synthetic --op ratio 可用 |

### 缺失模块（进入第二阶段的前置条件）

| 缺失项 | 优先级 | 阻塞性 |
|--------|--------|--------|
| 比价管线：线段→中枢→走势类型→背驰 | P1 | **非阻塞**——这是单标的管线的通用进度问题，不是跨标的特有缺口。一旦单标的管线完成这些步骤，比价管线自动获得 |
| 流动性条件显式检查 | P2 | 非阻塞——三层退化检测已覆盖大部分场景 |
| Flow(源→汇) 聚合数据结构 | P2 | **轻度阻塞**——流转关系定义的最终输出形态未代码化，但 VertexFlowState 已提供等价信息 |
| check_conservation() 显式函数 | P3 | 非阻塞——守恒是数学必然 |
| VertexFlowState 源/汇/中性显式分类 | P3 | 非阻塞——net_flow 正负已隐式表达 |
| flow_relation_v1.md 规范文件 | P3 | 非阻塞——定义文件已足够清晰 |

### 结论

跨标的三条定义的**核心管线已贯通**：从等价对验证→比价K线→标准管线→资本流转语义→四矩阵聚合→共振检测→时间序列，形成完整的数据流。129个单元测试和E2E测试验证了管线的正确性。

主要遗留问题集中在：
1. **单标的管线未完成的部分**（中枢/走势类型/背驰）自动影响比价管线——这不是跨标的特有缺口
2. **流转关系的最终输出形态**（Flow 数据结构）未代码化——功能等价但接口缺失
3. **等价关系的流动性条件**未显式实现——三层退化检测已部分覆盖

第二阶段的基础设施**已就位**，可以在单标的管线进一步完善后自然扩展。
