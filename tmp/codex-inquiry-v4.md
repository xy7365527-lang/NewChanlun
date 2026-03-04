# 全球资本流转 v4 代码层异质审查报告

**审查模式**: review（代码-理论对齐 + 架构可行性）
**审查对象**: global_capital_flow_v4.txt
**代码基线**: equivalence.py, capital_flow.py, matrix_topology.py, a_macd.py
**定义基线**: dengjia.md v1.1, liuzhuan.md v1.0, level_recursion.md v1.0
**日期**: 2026-02-27

---

## 一、代码-理论对齐审查

### 1.1 §2.5 等价关系级别依赖 vs equivalence.py

**v4 理论声明**：
> "两个资产在递归层级L上等价，当且仅当两者在层级L上都有已完成的走势类型。"

**现有代码实际行为**（equivalence.py validate_pair）：

validate_pair 是静态数据质量检查，输入是两个 DataFrame，输出是 ValidationResult。它完全不知道：
- 递归层级 level_id
- 任何一端资产在该层级上是否有已完成走势（settled Move）

**对齐缺口**：v4 §2.5 引入的"级别L上等价"是动态走势状态条件，与现有 C-1/C-2/C-3 的静态数据质量条件在本体论层面不同。

两层条件的区分：
1. 数据层等价（现有 C-1/C-2/C-3）：比价K线值不值得看 → 第一层分析的前提
2. 结构层等价（v4 新增）：两端在同级别有已完成走势 → 第二层（K4关系图）的前提

这两层不是替代关系，是叠加关系。

**代码改动方案**：

不修改 validate_pair（它处理第一层，无条件合法）。新建独立函数：



**关键阻塞**：RecursiveOrchestrator 目前处理单标的 K 线序列。K4 层需要同时管理 4 个资产的走势状态，并按 AssetVertex 索引查询。这个"多资产走势状态注册表"是目前完全缺失的基础设施。

**改动量评估**：
- validate_pair 本身：不改
- 新建 MoveRegistry Protocol：小（约15行）
- 新建 is_pair_valid_at_level：小（约10行）
- 将 RecursiveOrchestrator 输出适配为 MoveRegistry：中到大

---

### 1.2 §4.1 log-MACD vs a_macd.py

**v4 理论声明**：
> "MACD(ln A/C) = MACD(ln A/B) + MACD(ln B/C)，这是线性性直接给的。"

**现有代码**（a_macd.py compute_macd）：直接在价格域操作，没有 log 变换。

**可加性验证（数学层面）**：

log-MACD 的线性可加性严格成立：
- ln(A/C) = ln(A/B) + ln(B/C)（对数基本性质，永真）
- EMA 是线性算子：EMA(x + y) = EMA(x) + EMA(y)
- 因此：MACD(ln A/C) = MACD(ln A/B) + MACD(ln B/C) ✓

这不是近似，是精确等式。

**实现方案（最小改动）**：



**改动量**：小（约15行，不破坏现有接口）。

**注意事项**：
- make_ratio_kline 输出的 close 列是正数，np.log 可以直接应用
- EMA 初始化效应：序列前几十根 bar 的 log-MACD 值不稳定，跨边可加性在序列足够长后才精确成立
- _compute_macd_dynamics（equivalence.py 中用于退化检测）继续使用价格域 MACD，不受影响

---

### 1.3 §4.3 UNKNOWN 状态 vs FlowDirection 枚举

**v4 理论声明**：
> "E/C在第3层标记为unknown——不是'不存在'，而是'代数约束在此层级不可用'"

**现有代码**（capital_flow.py）：FlowDirection 有三个值：A_TO_B、B_TO_A、EQUILIBRIUM。

**语义区分（关键）**：
- EQUILIBRIUM：走势已确定为盘整（中枢内，方向未定但走势类型已知）
- UNKNOWN（缺失）：走势尚未完成，或两端不满足级别L等价条件，不参与K4关系层

两者在 _flow_contribution 中都贡献 0，但含义完全不同：
- EQUILIBRIUM = 确定性信息（走势类型已知）
- UNKNOWN = 不确定性（走势类型未知）

下游消费者（配置读取）必须区分这两种 0。

**改动方案**：



**改动量**：小（增加一个枚举值）。但需要检查所有使用 FlowDirection 的地方是否需要处理 UNKNOWN 分支。

---

## 二、架构可行性

### 2.1 K4 关系层需要的新模块

当前代码只有第一层基础设施：

| 现有模块 | 功能 | 层级 |
|---------|------|------|
| equivalence.py | pair-level 静态验证 | 第一层 |
| capital_flow.py | pair-level 笔到流转映射 | 第一层 |
| matrix_topology.py | 拓扑结构（静态） | 拓扑骨架 |
| a_macd.py | 价格域 MACD | 第一层 |

第二层需要新建：

| 新模块 | 功能 | 优先级 |
|--------|------|--------|
| move_registry.py | 多资产走势状态注册表（AssetVertex → level_id → settled?） | P0 |
| k4_level_scanner.py | 级别扫描：确定哪些边在级别L上合法 | P0 |
| flow_relation.py | 顶点聚合 + 共振检测（liuzhuan.md 已规划） | P1 |
| k4_configuration.py | 27种配置编码 + 派生边约束范围 | P1 |
| k4_disambiguator.py | 消歧：独立边状态 + 派生边观测 → 唯一全局读取 | P2 |

### 2.2 级别扫描的实现

MoveRegistry Protocol（新建）：

    class MoveRegistry(Protocol):
        def has_settled_move(self, vertex: AssetVertex, level_id: int) -> bool: ...
        def get_settled_move_type(self, vertex: AssetVertex, level_id: int) -> MoveType | None: ...

get_valid_subgraph（新建于 k4_level_scanner.py）：

    def get_valid_subgraph(matrix, level_id, registry):
        return frozenset(
            edge for edge in matrix.edges
            if registry.has_settled_move(edge.vertex_a, level_id)
            and registry.has_settled_move(edge.vertex_b, level_id)
        )

子图大小（0-6条边）本身就是信息：
- 0条：该级别上K4完全不可读
- 1-5条：局部图，可做局部一致性检验
- 6条：完整K4，全局流场可读

### 2.3 27种配置的编码方案

建议查表方案（不是运行时计算）。27种配置是有限且固定的，查表比运行时推导更可靠、更易测试。

    class EdgeState(Enum):
        UP = +
        DOWN = -
        FLAT = 0

    @dataclass(frozen=True)
    class DerivedEdgeConstraint:
        allowed: frozenset[EdgeState]
        requires_observation: bool
        note: str = 

    # CONFIG_TABLE: dict[(E/$, C/$, R/$), {派生边名: 约束}]
    # 27种配置全部预计算

---

## 三、定义文件更新需求

### 3.1 dengjia.md：结构层等价不是 C-4

v4 §2.5 的结构层等价不应加为 C-4。理由：
- C-1/C-2/C-3 是数据层条件（静态，一次性检查）
- 结构层等价是 K4 关系层条件（动态，随走势演进变化）

把它加为 C-4 会混淆两个层面。正确做法：
- dengjia.md 保持 C-1/C-2/C-3 不变
- 新增结构层等价（K4关系层专用）小节，明确标注这不是 C-4

### 3.2 liuzhuan.md：加法守恒 vs 乘法恒等式

v4 §2.3 说严格的守恒是乘法形式——这个表述有误导性。

两者是不同层面的约束：

| 约束 | 形式 | 层面 | 性质 |
|------|------|------|------|
| 乘法恒等式 | A/B × B/C × C/A ≡ 1 | 价格水平（代数） | 永真，不携带信息 |
| 加法守恒 | Σnet(V) = 0 | 流场方向（物理） | 可破缺，破缺是信号 |

乘法恒等式不是更严格的守恒，而是完全不同层面的约束。liuzhuan.md 的加法守恒才是真正的守恒约束（可以被破缺，破缺有信号意义）。

建议：liuzhuan.md 不改。v4 §2.3 修正措辞，明确区分两种约束的层面。

### 3.3 需要新建 k4_configuration.md

27种配置的完整枚举、派生边约束范围、消歧流程——这些内容既不属于 dengjia.md（筛选层），也不属于 liuzhuan.md（流场层），而是 K4 关系层的核心内容。

建议新建 .chanlun/definitions/k4_configuration.md。

---

## 四、测试策略

### 4.1 log-MACD 可加性测试

验证 MACD(ln A/C) = MACD(ln A/B) + MACD(ln B/C)：

    def test_log_macd_additivity():
        n = 200
        rng = np.random.default_rng(42)
        price_a = np.cumprod(1 + rng.normal(0, 0.01, n))
        price_b = np.cumprod(1 + rng.normal(0, 0.01, n))
        price_c = np.cumprod(1 + rng.normal(0, 0.01, n))
        df_ab = pd.DataFrame({close: price_a / price_b})
        df_bc = pd.DataFrame({close: price_b / price_c})
        df_ac = pd.DataFrame({close: price_a / price_c})
        macd_ab = compute_log_macd(df_ab)[hist]
        macd_bc = compute_log_macd(df_bc)[hist]
        macd_ac = compute_log_macd(df_ac)[hist]
        # 跳过前50根（EMA初始化效应）
        np.testing.assert_allclose(
            macd_ac.iloc[50:].values,
            (macd_ab + macd_bc).iloc[50:].values,
            rtol=1e-10,
        )

### 4.2 K4 配置分类测试

    def test_config_table_completeness():
        from itertools import product
        all_configs = list(product(EdgeState, repeat=3))
        assert len(all_configs) == 27
        for config in all_configs:
            assert config in CONFIG_TABLE

    def test_mode_d_requires_observation():
        # 模式D：E/$=+, C/$=+, R/$=+ → 派生边需观测
        config = (EdgeState.UP, EdgeState.UP, EdgeState.UP)
        for derived_edge in [E/C, E/R, C/R]:
            assert CONFIG_TABLE[config][derived_edge].requires_observation

### 4.3 结构层等价测试

    def test_is_pair_valid_at_level_both_settled():
        registry = MockMoveRegistry({(EQUITY, 2): True, (COMMODITY, 2): True})
        assert is_pair_valid_at_level(EQUITY, COMMODITY, 2, registry) is True

    def test_is_pair_valid_at_level_one_unsettled():
        registry = MockMoveRegistry({(EQUITY, 2): True, (COMMODITY, 2): False})
        assert is_pair_valid_at_level(EQUITY, COMMODITY, 2, registry) is False

---

## 五、总体评价

### v4 工程可落地性

第一层（单条比价线跑缠论）：完全可落地。现有代码已支持，改动量小：
- 新增 compute_log_macd（15行）
- 新增 FlowDirection.UNKNOWN（1行 + 影响范围检查）

第二层（K4关系图）：需要较大新建工作，但架构清晰，没有根本性阻塞。关键路径是 MoveRegistry（多资产走势状态注册表）。

### 关键阻塞点（优先级排序）

P0（必须解决才能进入第二层）：

1. MoveRegistry：将 RecursiveOrchestrator 的单标的输出适配为多资产走势状态注册表。改动量：中到大。

2. FlowDirection.UNKNOWN：语义上必须区分盘整（走势已确定为均衡）和待定（走势未完成或级别不满足）。改动量：小，但影响范围需检查。

P1（第二层核心功能）：

3. compute_log_macd：约15行，不破坏现有接口。

4. flow_relation.py：liuzhuan.md 已规划，实现顶点聚合 + 共振检测。

5. k4_level_scanner.py：级别扫描，依赖 MoveRegistry。

P2（完整性）：

6. k4_configuration.py：27种配置编码，查表方案。

7. 定义文件更新：dengjia.md 增加结构层等价说明，liuzhuan.md 增加层面区分说明，新建 k4_configuration.md。

### 对齐度评分

| v4 理论改动 | 代码基础 | 改动量 | 阻塞？ |
|------------|---------|--------|--------|
| §2.5 等价关系级别依赖 | 无（需从零建 MoveRegistry） | 大 | P0 |
| §4.1 log-MACD | 有（a_macd.py 可扩展） | 小 | 否 |
| §4.3 UNKNOWN 状态 | 有（FlowDirection 可扩展） | 小 | P0（语义） |
| §5 配置约束命题（27种） | 无（需从零建） | 中 | P1 |
| §7 级别扫描前置步骤 | 有（RecursiveStack 已实现） | 中 | P0（依赖 MoveRegistry） |
| §4.1 第一层背驰判定 | 有（compute_macd 已实现） | 无 | 否 |

### v4 内部不一致：需修正

v4 §2.3 说加法守恒只在比价变动小时近似成立，严格的守恒是乘法形式。这个表述有误：

- 乘法恒等式 A/B × B/C × C/A ≡ 1 是价格水平的代数恒等式（永真），不是守恒律
- 加法守恒 Σnet(V) = 0 是流场方向的物理约束（资本不凭空产生），是定义性的，不是近似

两者在不同层面，不存在哪个更严格的问题。v4 §2.3 的近似表述需要修正，否则会误导 liuzhuan.md 的守恒约束实现（如果按乘法守恒实现，会得到一个永远返回 True 的函数——无用）。

建议 v4 §2.3 改为：
乘法恒等式是价格水平的代数约束（永真，用于配置分类的搜索空间缩减）。加法守恒是流场方向的物理约束（资本不凭空产生，可破缺，破缺是信号）。两者在不同层面，互补而非替代。

---

## 附：代码改动清单（按优先级）

P0：
  新建 src/newchan/move_registry.py
    - MoveRegistry Protocol
    - AssetMoveState dataclass
    - MultiAssetOrchestrator（管理4个资产的 RecursiveOrchestrator）
  修改 src/newchan/capital_flow.py
    - FlowDirection 增加 UNKNOWN = "待定"

P1：
  修改 src/newchan/a_macd.py
    - 新增 compute_log_macd()
  新建 src/newchan/k4_level_scanner.py
    - is_pair_valid_at_level()
    - get_valid_subgraph()
  新建 src/newchan/flow_relation.py（liuzhuan.md 已规划）
    - VertexFlowState
    - detect_resonance()
    - check_conservation()

P2：
  新建 src/newchan/k4_configuration.py
    - EdgeState 枚举
    - DerivedEdgeConstraint dataclass
    - CONFIG_TABLE（27种配置查表）
  更新 .chanlun/definitions/dengjia.md
    - 增加结构层等价说明（K4关系层专用，非 C-4）
  更新 .chanlun/definitions/liuzhuan.md
    - 增加乘法恒等式 vs 加法守恒的层面区分说明
  新建 .chanlun/definitions/k4_configuration.md
    - 27种配置完整枚举
    - 派生边约束范围
    - 消歧流程
