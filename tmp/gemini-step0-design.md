# Step 0 基础层实现方案

**模式**: derive（设计方案）
**对象**: EdgeFlowInput.level_id + aggregate_vertex_flows 参数化 + FlowDirection.UNKNOWN
**日期**: 2026-02-27
**依据**: global_capital_flow_v4.txt §2.5/§4.3, codex-inquiry-v4.md, session 2026-02-27-0039

---

## 一、EdgeFlowInput 加 level_id

### 1.1 精确字段定义

```python
@dataclass(frozen=True, slots=True)
class EdgeFlowInput:
    vertex_a: AssetVertex
    vertex_b: AssetVertex
    direction: FlowDirection
    level_id: int | None = None
```

**类型**: `int | None`
**默认值**: `None`
**语义**:
- `None` = 第一层分析（无级别约束，永远合法）
- `int` = 第二层 K4 关系图分析，在递归层级 `level_id` 上的方向判读

`level_id` 的值域不在 `EdgeFlowInput` 层面约束——它只是一个标签，语义由上游（MoveRegistry / k4_level_scanner）保证。`EdgeFlowInput` 不知道也不需要知道"这个 level_id 是否合法"。

### 1.2 __post_init__ 的影响

**不变**：只保留现有的自环检查。

```python
def __post_init__(self) -> None:
    if self.vertex_a == self.vertex_b:
        raise ValueError(f"自环不合法：{self.vertex_a}")
```

不增加对 `level_id` 的验证。理由：
- `level_id=None` 永远合法（第一层）
- `level_id=N`（任意整数）的合法性由调用者（k4_level_scanner）在构造前保证
- 在 dataclass 层面验证 level_id 的合法性需要引入 MoveRegistry 依赖，违反单一职责

### 1.3 向后兼容性

所有现有调用者不传 `level_id` 时，得到 `level_id=None`，行为完全不变。

现有调用点（需确认但预期无需改动）：
- `disambiguate_cash_signal`：内部构造 EdgeFlowInput 的地方（如有）
- 所有测试文件中的 `EdgeFlowInput(vertex_a=..., vertex_b=..., direction=...)` 构造

---

## 二、aggregate_vertex_flows 参数化边集合

### 2.1 当前问题的根源

```python
if len(edges) != 6:
    raise ValueError(f"需要恰好 6 条边，实际 {len(edges)} 条")
```

这个约束与 v4 §2.5 矛盾：

> "如果某个递归层级上只有三条边满足等价条件，你有一个三角形子图，可以做局部一致性检验"

K3 子图（3条边）、K2 子图（1-2条边）都是合法的部分图。强制要求 6 条边意味着：
- 无法处理 K4 不完整时的局部流场读取
- 无法在 level_id 过滤后对子图聚合

### 2.2 新接口签名

```python
def aggregate_vertex_flows(
    edges: list[EdgeFlowInput],
) -> list[VertexFlowState]:
    """聚合边集合的方向为 4 个顶点的流转状态。

    接受 1-6 条边的任意子集（完整 K4 或部分子图）。
    UNKNOWN 方向的边不得传入——调用者必须在调用前过滤。

    Parameters
    ----------
    edges : list[EdgeFlowInput]
        1-6 条边的流转方向输入。不得包含 FlowDirection.UNKNOWN。

    Returns
    -------
    list[VertexFlowState]
        4 个顶点的流转状态，按 AssetVertex 枚举顺序排列。
        未被任何传入边覆盖的顶点，net_flow=0，role=NEUTRAL。

    Raises
    ------
    ValueError
        边数为 0，或存在重复边，或包含 UNKNOWN 方向的边。
    """
```

**关键变化**：
- 移除 `len(edges) != 6` 检查
- 增加 `len(edges) < 1` 检查（空集无意义）
- 增加 UNKNOWN 方向检查（见第三节）

### 2.3 对 net flow 计算的影响（部分图语义）

net flow 计算逻辑不变：

```python
for vertex in AssetVertex:
    net = sum(_flow_contribution(e, vertex) for e in edges)
```

部分图的语义：
- 顶点 V 的 `net_flow` = 传入边集合中与 V 相连的边的贡献之和
- 未被任何传入边覆盖的顶点：`net_flow=0`，`role=NEUTRAL`
- 这是正确的——NEUTRAL 在部分图中意味着"在当前子图中无方向性流转"，不是"全图中无方向性流转"

**调用者的责任**：解读 `VertexFlowState` 时，需知道这是基于哪个子图计算的。`VertexFlowState` 本身不携带子图信息——这是上层（k4_level_scanner）的职责。

### 2.4 守恒约束 check_conservation 在部分图上的行为

`check_conservation` 不需要改动。

数学证明：对任意边子集，`Σnet(V) = 0` 恒成立。

证明：每条边 `e` 对 `vertex_a` 贡献 `c_a ∈ {+1, -1, 0}`，对 `vertex_b` 贡献 `-c_a`（方向相反），对其他顶点贡献 0。因此每条边对所有顶点的贡献之和为 0。对所有边求和，`Σnet(V) = 0`。

这是有向图的拓扑恒等式，与边数无关。`check_conservation` 在部分图上永远返回 `True`——这是正确的，不是 bug。

**注意**：`check_conservation` 返回 `False` 只在一种情况下发生：`_flow_contribution` 的实现有 bug（贡献不对称）。在正确实现下，它是一个不变量验证，不是业务逻辑。

---

## 三、FlowDirection.UNKNOWN

### 3.1 枚举值定义

在 `capital_flow.py` 的 `FlowDirection` 枚举中增加：

```python
class FlowDirection(Enum):
    A_TO_B = "A→B"       # 资本从 A 流向 B（比价下跌）
    B_TO_A = "B→A"       # 资本从 B 流向 A（比价上涨）
    EQUILIBRIUM = "均衡"  # 走势已确定为盘整（中枢内震荡，走势类型已知）
    UNKNOWN = "待定"      # 走势未完成，或两端不满足级别L等价条件（走势类型未知）
```

### 3.2 与 EQUILIBRIUM 的语义区分

| 维度 | EQUILIBRIUM | UNKNOWN |
|------|-------------|---------|
| 走势类型 | 已确定（盘整） | 未确定 |
| 信息量 | 携带确定性信息 | 不携带信息 |
| 参与 K4 关系层 | 是（贡献 0） | 否（必须过滤） |
| 来源 | 走势分析完成，判定为中枢内震荡 | 走势未完成，或级别L等价条件不满足 |
| 下游处理 | 正常参与聚合 | 在调用 aggregate_vertex_flows 前过滤掉 |

这两种"0贡献"在配置读取层面含义完全不同：
- EQUILIBRIUM 的 0 = "这条边确定无方向性流转"（配置表中的 `0` 列）
- UNKNOWN 的 0 = "这条边不参与当前级别的配置读取"（边不存在于子图中）

### 3.3 _flow_contribution 对 UNKNOWN 的处理

**结论：raise ValueError**

```python
def _flow_contribution(edge: EdgeFlowInput, vertex: AssetVertex) -> int:
    if edge.direction == FlowDirection.UNKNOWN:
        raise ValueError(
            f"UNKNOWN 方向的边不得传入 aggregate_vertex_flows："
            f"{edge.vertex_a.value}/{edge.vertex_b.value}。"
            f"调用者必须在调用前过滤 UNKNOWN 边。"
        )
    if edge.direction == FlowDirection.EQUILIBRIUM:
        return 0
    # ... 其余逻辑不变
```

理由：
- UNKNOWN 边不应出现在 `aggregate_vertex_flows` 的输入中
- 如果出现，说明调用者没有正确过滤——这是调用者的 bug，应该 fail loudly
- 返回 0 会静默地把 UNKNOWN 当作 EQUILIBRIUM 处理，掩盖调用者的错误
- 严格性原则：合约违反必须立即报错

### 3.4 _direction_value 对 UNKNOWN 的处理

同样 raise ValueError：

```python
def _direction_value(direction: FlowDirection) -> int:
    if direction == FlowDirection.UNKNOWN:
        raise ValueError(
            "UNKNOWN 方向无法映射为数值。"
            "调用者必须在调用前过滤 UNKNOWN 边。"
        )
    if direction == FlowDirection.A_TO_B:
        return 1
    if direction == FlowDirection.B_TO_A:
        return -1
    return 0  # EQUILIBRIUM
```

### 3.5 disambiguate_cash_signal 的适配

`disambiguate_cash_signal` 内部调用 `aggregate_vertex_flows`。

**不改动函数签名**，但在 docstring 中明确：

```python
def disambiguate_cash_signal(
    edge_inputs: list[EdgeFlowInput],
) -> CashSignalAnalysis:
    """现金边信号消歧：用纯资产子图校验现金边信号的可信度。

    Parameters
    ----------
    edge_inputs : list[EdgeFlowInput]
        边集合。不得包含 FlowDirection.UNKNOWN 的边——
        调用者必须在调用前过滤（UNKNOWN 边不参与消歧分析）。
    ...
    """
```

函数体不变——UNKNOWN 边的过滤是调用者的职责，`disambiguate_cash_signal` 只是透传给 `aggregate_vertex_flows`，后者会在遇到 UNKNOWN 时 raise。

---

## 四、测试策略

### 4.1 现有测试的兼容性

所有现有测试构造 `EdgeFlowInput` 时不传 `level_id`，得到 `level_id=None`，行为不变。

所有现有测试传入 6 条边给 `aggregate_vertex_flows`，行为不变（6条边仍然合法）。

**预期**：现有测试全部通过，无需修改。

### 4.2 新增测试用例

#### EdgeFlowInput.level_id

```python
def test_edge_flow_input_default_level_id():
    e = EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.CASH, FlowDirection.A_TO_B)
    assert e.level_id is None

def test_edge_flow_input_with_level_id():
    e = EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.CASH, FlowDirection.A_TO_B, level_id=3)
    assert e.level_id == 3

def test_edge_flow_input_level_id_zero():
    # level_id=0 合法（第0层）
    e = EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.CASH, FlowDirection.A_TO_B, level_id=0)
    assert e.level_id == 0
```

#### 部分图聚合

```python
def test_aggregate_partial_graph_3_edges():
    # K3 子图：EQUITY-CASH, COMMODITY-CASH, EQUITY-COMMODITY
    edges = [
        EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.CASH, FlowDirection.B_TO_A),
        EdgeFlowInput(AssetVertex.COMMODITY, AssetVertex.CASH, FlowDirection.A_TO_B),
        EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.COMMODITY, FlowDirection.B_TO_A),
    ]
    states = aggregate_vertex_flows(edges)
    assert len(states) == 4
    # REAL_ESTATE 未被任何边覆盖，net_flow=0
    re_state = next(s for s in states if s.vertex == AssetVertex.REAL_ESTATE)
    assert re_state.net_flow == 0
    assert re_state.role == FlowRole.NEUTRAL

def test_aggregate_single_edge():
    edges = [EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.CASH, FlowDirection.B_TO_A)]
    states = aggregate_vertex_flows(edges)
    assert len(states) == 4

def test_aggregate_empty_raises():
    with pytest.raises(ValueError, match="至少需要 1 条边"):
        aggregate_vertex_flows([])
```

#### UNKNOWN 边排除

```python
def test_aggregate_unknown_edge_raises():
    edges = [
        EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.CASH, FlowDirection.UNKNOWN),
    ]
    with pytest.raises(ValueError, match="UNKNOWN"):
        aggregate_vertex_flows(edges)

def test_direction_value_unknown_raises():
    with pytest.raises(ValueError, match="UNKNOWN"):
        _direction_value(FlowDirection.UNKNOWN)
```

#### 守恒约束在部分图上

```python
def test_conservation_holds_for_partial_graph():
    # 任意子集都满足守恒
    edges = [
        EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.CASH, FlowDirection.B_TO_A),
        EdgeFlowInput(AssetVertex.COMMODITY, AssetVertex.CASH, FlowDirection.A_TO_B),
    ]
    states = aggregate_vertex_flows(edges)
    assert check_conservation(states) is True
```

---

## 五、向后兼容性总结

### 不需要改动的调用者

- 所有传入 6 条非 UNKNOWN 边的调用
- 所有不传 `level_id` 的 `EdgeFlowInput` 构造
- `check_conservation` 的所有调用
- `detect_resonance` 的所有调用
- `extract_flow_relations` 的所有调用

### 必须适配的调用者

目前**无**——因为：
1. `level_id` 有默认值 `None`，现有构造不受影响
2. `aggregate_vertex_flows` 放宽了约束（从"必须6条"到"1-6条"），现有6条调用仍然合法
3. `FlowDirection.UNKNOWN` 是新增枚举值，现有代码不会产生它

**未来**调用者（k4_level_scanner）在构造 `EdgeFlowInput` 时需要：
- 传入 `level_id`（第二层分析时）
- 过滤掉 UNKNOWN 边后再调用 `aggregate_vertex_flows`

---

## 六、v4 理论与代码之间的不一致（发现）

### 6.1 flow_relation.py 已存在

Codex 报告（§2.1）将 `flow_relation.py` 列为"需要新建"（P1）。但实际上 `flow_relation.py` 已经存在，且已实现：
- `EdgeFlowInput`
- `aggregate_vertex_flows`
- `detect_resonance`
- `extract_flow_relations`
- `check_conservation`
- `disambiguate_cash_signal`

Codex 报告的 P1 清单需要更新：`flow_relation.py` 不是新建，是在现有文件上修改。

### 6.2 aggregate_vertex_flows 的 6 条边约束是代码层 bug

v4 §2.5 明确说明部分图合法，但代码强制要求 6 条边。这是代码层的实现缺陷，不是理论层的问题。Step 0 修复这个缺陷。

### 6.3 EQUILIBRIUM vs UNKNOWN 的语义区分在代码中完全缺失

`FlowDirection` 枚举没有 UNKNOWN，导致"走势已确定为盘整"和"走势未完成"在代码层无法区分。这是 v4 §4.3 指出的缺口，Step 0 补齐。

---

## 七、改动文件清单

| 文件 | 改动类型 | 改动内容 |
|------|---------|---------|
| `src/newchan/capital_flow.py` | 修改 | `FlowDirection` 增加 `UNKNOWN = "待定"` |
| `src/newchan/flow_relation.py` | 修改 | `EdgeFlowInput` 加 `level_id: int \| None = None`；`aggregate_vertex_flows` 移除 6 条边约束，加空集检查；`_flow_contribution` 加 UNKNOWN raise；`_direction_value` 加 UNKNOWN raise；`disambiguate_cash_signal` docstring 更新 |
| `tests/test_flow_relation.py` | 修改 | 新增上述测试用例 |

**不改动**：
- `matrix_topology.py`（拓扑结构不变）
- `equivalence.py`（第一层静态验证不变）
- `a_macd.py`（Step 1 并行工位的任务）
- 任何定义文件（Step 0 只改代码层）
