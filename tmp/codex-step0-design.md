# Step 0 基础层实现方案

**模式**: review（方案设计）
**审查工位**: Codex 异质审查（claude-sonnet-4-6 代理）
**对象**: EdgeFlowInput + aggregate_vertex_flows + FlowDirection.UNKNOWN
**代码基线**: flow_relation.py, capital_flow.py, matrix_topology.py
**理论基线**: global_capital_flow_v4.txt §2.5 / §4.3
**日期**: 2026-02-27

---

## 一、EdgeFlowInput 加 level_id

### 1.1 精确字段定义

```python
@dataclass(frozen=True, slots=True)
class EdgeFlowInput:
    vertex_a: AssetVertex
    vertex_b: AssetVertex
    direction: FlowDirection
    level_id: int | None = None  # 新增
```

**类型**: `int | None`
**默认值**: `None`
**语义**:
- `None` — 不关联特定递归层级，属于第一层分析（单条比价线跑缠论，无条件合法）
- `int` — 该边的方向判读来自递归层级 `level_id`，参与 K4 Layer 2 分析

`level_id` 的含义是"这条边的走势类型在哪个递归层级上已完成"，不是"这条边属于哪个K线周期"。递归层级是缠论递归构造的层数（笔=0，线段=1，中枢=2，走势类型=3，...），不是日线/周线等周期概念。

### 1.2 对 __post_init__ 的影响

无需修改。现有检查：

```python
def __post_init__(self) -> None:
    if self.vertex_a == self.vertex_b:
        raise ValueError(f"自环不合法：{self.vertex_a}")
```

`level_id` 不需要在 `__post_init__` 中验证：
- `None` 是合法的（第一层分析）
- 负整数在理论上无意义，但 level_id 的合法性由调用者（MoveRegistry / k4_level_scanner）负责，不在 EdgeFlowInput 层验证

### 1.3 向后兼容性

`level_id=None` 是默认值，所有现有调用者不传 `level_id` 时行为完全不变。

现有调用者（flow_relation.py 内部、测试文件）构造 EdgeFlowInput 时不传 level_id，得到 `level_id=None` 的实例，所有下游函数（aggregate_vertex_flows、_flow_contribution、disambiguate_cash_signal）不读取 level_id，行为不变。

**不需要改动的现有调用者**：所有现有构造 EdgeFlowInput 的代码。

---

## 二、aggregate_vertex_flows 参数化边集合

### 2.1 当前问题

```python
# flow_relation.py:186
if len(edges) != 6:
    raise ValueError(f"需要恰好 6 条边，实际 {len(edges)} 条")
```

v4 §2.5 明确：K4 在特定递归层级上可能只有部分边满足等价条件（两端都有已完成走势）。此时合法子图是 1-5 条边的部分图。硬编码 6 条边使得部分图聚合完全不可用——这是整个第二层数据结构的缺失，不是边界情况。

### 2.2 新接口签名

```python
def aggregate_vertex_flows(
    edges: list[EdgeFlowInput],
) -> list[VertexFlowState]:
    """聚合边集合的方向为 4 个顶点的流转状态。

    Parameters
    ----------
    edges : list[EdgeFlowInput]
        1 到 6 条边的流转方向输入。可以是完整 K4 图（6条）或任意子图（1-5条）。
        所有边的 direction 必须是已确定状态（A_TO_B / B_TO_A / EQUILIBRIUM）。
        UNKNOWN 方向的边必须在调用前过滤掉。

    Returns
    -------
    list[VertexFlowState]
        4 个顶点的流转状态，按 AssetVertex 枚举顺序排列。
        net_flow 仅反映传入边集合的贡献，不代表完整 K4 图的净流量。

    Raises
    ------
    ValueError
        边数为 0，或存在重复边，或存在 UNKNOWN 方向的边。
    """
    if len(edges) == 0:
        raise ValueError("至少需要 1 条边")
    if len(edges) > 6:
        raise ValueError(f"K4 图最多 6 条边，实际 {len(edges)} 条")

    # 检查 UNKNOWN 方向
    for e in edges:
        if e.direction == FlowDirection.UNKNOWN:
            raise ValueError(
                f"边 {e.vertex_a.value}/{e.vertex_b.value} 方向为 UNKNOWN，"
                "必须在调用前过滤"
            )

    # 检查重复
    seen: set[frozenset[AssetVertex]] = set()
    for e in edges:
        key = frozenset([e.vertex_a, e.vertex_b])
        if key in seen:
            raise ValueError(f"重复边：{e.vertex_a.value}/{e.vertex_b.value}")
        seen.add(key)

    states: list[VertexFlowState] = []
    for vertex in AssetVertex:
        net = sum(_flow_contribution(e, vertex) for e in edges)
        states.append(
            VertexFlowState(
                vertex=vertex,
                net_flow=net,
                strength=_classify_resonance(net),
                role=_classify_role(net),
            )
        )
    return states
```

### 2.3 部分图的 net flow 语义

部分图（1-5条边）的 net_flow 含义：**仅来自传入边集合的净流量贡献**，不是完整 K4 图的净流量。

调用者必须理解这个语义差异：
- 完整图（6条边）：net_flow 是完整流场的顶点净流量
- 部分图（k条边）：net_flow 是 k 条边构成的子图的顶点净流量

这不是近似，是精确的——子图的净流量就是子图的净流量，不是完整图的近似。

### 2.4 守恒约束在部分图上的行为

**结论：check_conservation 在任意边集合（1-6条）上永远返回 True。**

证明：每条边 (A→B) 对顶点 A 贡献 -1，对顶点 B 贡献 +1，对其他顶点贡献 0。因此每条边对 Σnet(V) 的贡献是 (-1) + (+1) = 0。k 条边的总贡献是 k × 0 = 0。

推论：
- check_conservation 在部分图上是恒真命题（tautology），不携带信息
- check_conservation 的信号意义（检测数据错误）**仅在完整 6 条边图上有意义**
- 在部分图上调用 check_conservation 不会出错，但结果永远是 True，调用者不应依赖此结果做判断

**建议**：在 check_conservation 的 docstring 中注明此行为，避免调用者误用。

### 2.5 disambiguate_cash_signal 的适配

`disambiguate_cash_signal` 是第一层分析函数，语义上要求完整 6 条边（需要现金边 + 纯资产子图）。去掉 aggregate_vertex_flows 的 6 条边限制后，需要在 disambiguate_cash_signal 内部显式验证：

```python
def disambiguate_cash_signal(
    edge_inputs: list[EdgeFlowInput],
) -> CashSignalAnalysis:
    """现金边信号消歧：用纯资产子图校验现金边信号的可信度。

    要求恰好 6 条边（完整 K4 图）。这是第一层分析函数，
    不适用于部分图——部分图缺少现金边或纯资产边时消歧无意义。
    """
    if len(edge_inputs) != 6:
        raise ValueError(
            f"disambiguate_cash_signal 要求恰好 6 条边，实际 {len(edge_inputs)} 条。"
            "部分图消歧无意义（可能缺少现金边或纯资产边）。"
        )
    states = aggregate_vertex_flows(edge_inputs)
    # ... 其余逻辑不变
```

这是必要的：aggregate_vertex_flows 放开了边数限制，但 disambiguate_cash_signal 的语义本身要求完整图。不加此检查会导致静默错误（现金边缺失时 cash_net=0，误判为 neutral）。

---

## 三、FlowDirection.UNKNOWN

### 3.1 枚举值定义

```python
# capital_flow.py
class FlowDirection(Enum):
    A_TO_B = "A→B"
    B_TO_A = "B→A"
    EQUILIBRIUM = "均衡"
    UNKNOWN = "待定"  # 新增
```

### 3.2 与 EQUILIBRIUM 的语义区分

| | EQUILIBRIUM | UNKNOWN |
|---|---|---|
| 走势类型 | 已确定（盘整，中枢内震荡） | 未完成，或级别等价条件不满足 |
| 信息量 | 确定性信息（方向已知为均衡） | 不确定性（方向未知） |
| 在 _flow_contribution 中 | 返回 0（确定性中性） | 不应到达此函数 |
| 参与 aggregate_vertex_flows | 合法（贡献 0） | 非法（调用前必须过滤） |
| 参与 K4 Layer 2 | 合法（走势类型已知） | 非法（走势类型未知） |

EQUILIBRIUM 和 UNKNOWN 在 _flow_contribution 中都贡献 0，但含义完全不同。下游消费者（k4_configuration.py 的配置读取）必须区分：
- EQUILIBRIUM = 配置表中的 "0"（盘整，走势类型已定）
- UNKNOWN = 该边不参与配置读取（走势类型未定）

### 3.3 _flow_contribution 对 UNKNOWN 的处理

**结论：raise ValueError，不返回 0。**

理由：UNKNOWN 边不应进入 aggregate_vertex_flows。如果到达 _flow_contribution，说明调用者没有在上游过滤 UNKNOWN 边——这是编程错误，应该快速失败而不是静默返回 0（静默返回 0 会把 UNKNOWN 误当作 EQUILIBRIUM 处理，产生错误的流场读取）。

```python
def _flow_contribution(edge: EdgeFlowInput, vertex: AssetVertex) -> int:
    if edge.direction == FlowDirection.UNKNOWN:
        raise ValueError(
            f"边 {edge.vertex_a.value}/{edge.vertex_b.value} 方向为 UNKNOWN，"
            "不应参与流场聚合。调用前请过滤 UNKNOWN 边。"
        )
    if edge.direction == FlowDirection.EQUILIBRIUM:
        return 0
    # ... A_TO_B / B_TO_A 逻辑不变
```

注意：aggregate_vertex_flows 已在入口处检查 UNKNOWN（见 §2.2），_flow_contribution 的检查是防御性的第二道保障。

### 3.4 _direction_value 对 UNKNOWN 的处理

`_direction_value` 用于 disambiguate_cash_signal 的纯资产子图方差计算。

**结论：raise ValueError。**

理由与 _flow_contribution 相同——disambiguate_cash_signal 是第一层函数，不应接收 UNKNOWN 边。

```python
def _direction_value(direction: FlowDirection) -> int:
    if direction == FlowDirection.UNKNOWN:
        raise ValueError("UNKNOWN 方向不能转换为数值，调用前请过滤")
    if direction == FlowDirection.A_TO_B:
        return 1
    if direction == FlowDirection.B_TO_A:
        return -1
    return 0  # EQUILIBRIUM
```

### 3.5 disambiguate_cash_signal 的适配

`disambiguate_cash_signal` 调用 `aggregate_vertex_flows`（已在入口检查 UNKNOWN）和 `_direction_value`（已检查 UNKNOWN）。因此 disambiguate_cash_signal 本身不需要额外的 UNKNOWN 检查——上游已覆盖。

但需要加 6 条边检查（见 §2.5）。

---

## 四、测试策略

### 4.1 现有测试的兼容性

所有现有测试构造 EdgeFlowInput 时不传 level_id，得到 `level_id=None`，行为不变。

现有测试调用 aggregate_vertex_flows 时传 6 条边，仍然合法（6 在 1-6 范围内）。

**预期**：现有测试全部通过，无需修改。

### 4.2 新增测试用例

#### EdgeFlowInput level_id

```python
def test_edge_flow_input_level_id_default_none():
    e = EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.CASH, FlowDirection.A_TO_B)
    assert e.level_id is None

def test_edge_flow_input_level_id_set():
    e = EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.CASH, FlowDirection.A_TO_B, level_id=3)
    assert e.level_id == 3

def test_edge_flow_input_frozen():
    e = EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.CASH, FlowDirection.A_TO_B, level_id=2)
    with pytest.raises(FrozenInstanceError):
        e.level_id = 3
```

#### 部分图聚合

```python
def test_aggregate_partial_graph_triangle():
    # 三角形子图（3条边）
    edges = [
        EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.CASH, FlowDirection.A_TO_B),
        EdgeFlowInput(AssetVertex.COMMODITY, AssetVertex.CASH, FlowDirection.B_TO_A),
        EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.COMMODITY, FlowDirection.A_TO_B),
    ]
    states = aggregate_vertex_flows(edges)
    assert len(states) == 4
    # EQUITY: -1（流出到CASH）-1（流出到COMMODITY）= -2 → SOURCE
    equity_state = next(s for s in states if s.vertex == AssetVertex.EQUITY)
    assert equity_state.net_flow == -2
    assert equity_state.role == FlowRole.SOURCE

def test_aggregate_single_edge():
    edges = [EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.CASH, FlowDirection.A_TO_B)]
    states = aggregate_vertex_flows(edges)
    assert len(states) == 4

def test_aggregate_empty_raises():
    with pytest.raises(ValueError, match="至少需要 1 条边"):
        aggregate_vertex_flows([])

def test_aggregate_seven_edges_raises():
    # 构造7条边（含重复）
    with pytest.raises(ValueError):
        aggregate_vertex_flows([...] * 7)
```

#### UNKNOWN 边排除

```python
def test_aggregate_unknown_edge_raises():
    edges = [
        EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.CASH, FlowDirection.UNKNOWN),
    ]
    with pytest.raises(ValueError, match="UNKNOWN"):
        aggregate_vertex_flows(edges)

def test_flow_contribution_unknown_raises():
    edge = EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.CASH, FlowDirection.UNKNOWN)
    with pytest.raises(ValueError, match="UNKNOWN"):
        _flow_contribution(edge, AssetVertex.EQUITY)
```

#### 守恒约束部分图（恒真）

```python
def test_conservation_partial_graph_always_true():
    # 任意部分图守恒恒成立
    for n_edges in range(1, 7):
        edges = list(itertools.islice(all_six_edges(), n_edges))
        states = aggregate_vertex_flows(edges)
        assert check_conservation(states) is True
```

#### disambiguate_cash_signal 6条边检查

```python
def test_disambiguate_requires_six_edges():
    edges = [
        EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.CASH, FlowDirection.A_TO_B),
    ]
    with pytest.raises(ValueError, match="6 条边"):
        disambiguate_cash_signal(edges)
```

---

## 五、向后兼容性总结

### 不需要改动的现有调用者

- 所有构造 `EdgeFlowInput` 的代码（level_id 默认 None）
- 所有调用 `aggregate_vertex_flows` 传 6 条边的代码
- 所有调用 `disambiguate_cash_signal` 传 6 条边的代码
- 所有使用 `FlowDirection.A_TO_B / B_TO_A / EQUILIBRIUM` 的代码

### 必须适配的现有调用者

无。Step 0 的所有改动都是向后兼容的：
- `level_id` 有默认值 `None`
- `aggregate_vertex_flows` 放宽了约束（6→1-6），不是收紧
- `FlowDirection.UNKNOWN` 是新增枚举值，不影响现有枚举值的使用
- `disambiguate_cash_signal` 新增 6 条边检查——现有调用者本来就传 6 条边，不受影响

---

## 六、v4 理论与代码不一致点

### 6.1 aggregate_vertex_flows 硬编码（已在本方案解决）

v4 §2.5 要求部分图聚合，现有代码硬编码 6 条边。本方案解决。

### 6.2 EdgeFlowInput 缺少 level_id（已在本方案解决）

v4 §4.3 要求区分不同递归层级的方向判读，现有 EdgeFlowInput 不携带 level_id。本方案解决。

### 6.3 FlowDirection 缺少 UNKNOWN（已在本方案解决）

v4 §4.3 要求区分 EQUILIBRIUM（走势已定为盘整）和 UNKNOWN（走势未完成）。本方案解决。

### 6.4 check_conservation 在部分图上的语义（本方案新发现）

v4 §2.3 说守恒约束是"资本不凭空产生也不凭空消失"的形式表达，破缺是信号。但在部分图上，守恒是恒真命题，不携带信号。

**影响**：check_conservation 的信号意义仅在完整 6 条边图上有效。调用者在部分图上调用 check_conservation 不会出错，但结果永远是 True，不应用于数据质量判断。

建议在 check_conservation 的 docstring 中注明：部分图（<6条边）上守恒恒成立，信号意义仅在完整图上有效。

---

## 七、改动清单（精确）

### capital_flow.py

```
FlowDirection 枚举：增加 UNKNOWN = "待定"
```

### flow_relation.py

```
EdgeFlowInput：增加字段 level_id: int | None = None

aggregate_vertex_flows：
  - 删除 if len(edges) != 6: raise ValueError(...)
  - 增加 if len(edges) == 0: raise ValueError("至少需要 1 条边")
  - 增加 if len(edges) > 6: raise ValueError(...)
  - 增加 UNKNOWN 方向检查循环
  - docstring 更新（说明部分图语义）

_flow_contribution：
  - 增加 UNKNOWN 分支 → raise ValueError

_direction_value：
  - 增加 UNKNOWN 分支 → raise ValueError

disambiguate_cash_signal：
  - 增加 if len(edge_inputs) != 6: raise ValueError(...)
  - docstring 更新（说明要求完整图）

check_conservation：
  - docstring 更新（说明部分图上恒真，信号意义仅在完整图）
```

### 测试文件（新增）

```
tests/test_flow_relation_step0.py（或追加到现有测试文件）：
  - test_edge_flow_input_level_id_*（3个）
  - test_aggregate_partial_graph_*（4个）
  - test_aggregate_unknown_edge_raises
  - test_flow_contribution_unknown_raises
  - test_conservation_partial_graph_always_true
  - test_disambiguate_requires_six_edges
```

---

## 八、严格性自检

| 检查项 | 结论 |
|--------|------|
| 每个改动有明确语义理由 | ✓ |
| 接口签名精确到类型注解 | ✓ |
| 无补丁思维（"先加字段后面再说"） | ✓ level_id 语义明确（第一层/第二层区分） |
| v4 理论与代码不一致点全部指出 | ✓ 4处，其中3处本方案解决，1处新发现 |
| disambiguate_cash_signal 内部调用适配 | ✓ 需加6条边检查，理由明确 |
| 向后兼容性完整分析 | ✓ 无需改动的现有调用者已列出 |
| 守恒约束在部分图上的行为 | ✓ 恒真，已分析并建议文档化 |
