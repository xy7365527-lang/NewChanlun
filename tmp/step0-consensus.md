# Step 0 基础层共识方案

**来源**: Gemini 方案 + Codex 方案交叉审查
**日期**: 2026-02-27
**状态**: 共识达成，可直接实现

---

## 共识点（两份方案完全一致）

1. `EdgeFlowInput` 加 `level_id: int | None = None`，`__post_init__` 不变
2. `aggregate_vertex_flows` 移除 `!= 6` 约束
3. `FlowDirection.UNKNOWN = "待定"` 新增枚举值
4. `_flow_contribution` 遇 UNKNOWN → raise ValueError
5. `_direction_value` 遇 UNKNOWN → raise ValueError
6. 守恒约束在部分图上恒成立（有向图拓扑恒等式），`check_conservation` 不需要改动逻辑，只更新 docstring
7. 向后兼容：现有调用者无需改动

---

## 分歧与裁决

### 分歧1：disambiguate_cash_signal 是否需要显式 6 条边检查

| | Gemini | Codex |
|---|---|---|
| 方案 | 不加，依赖调用者职责 | 显式加 `if len(edge_inputs) != 6: raise` |

**裁决：Codex 正确。**

理由：`disambiguate_cash_signal` 语义本身要求完整图（需要现金边 + 纯资产子图）。传入 3 条非 UNKNOWN 边时 `aggregate_vertex_flows` 不会 raise，但消歧结果是错的——现金边缺失时 `cash_net=0`，误判为 `neutral`。这是静默错误，必须在函数入口显式拦截。

### 分歧2：aggregate_vertex_flows 是否需要 `> 6` 上界检查

| | Gemini | Codex |
|---|---|---|
| 方案 | 只检查 `< 1` | 同时检查 `== 0` 和 `> 6` |

**裁决：Codex 正确。**

理由：K4 图最多 6 条边，传入 7 条边是调用者 bug，应该 fail loudly。重复边检查（`seen` set）会捕获重复，但不会捕获"7 条不同边"的情况（K4 只有 6 条不同边，7 条必然有重复，所以实际上重复检查会捕获——但显式上界检查更清晰，错误信息更直接）。

---

## 最终接口规格

### capital_flow.py

```python
class FlowDirection(Enum):
    A_TO_B = "A→B"
    B_TO_A = "B→A"
    EQUILIBRIUM = "均衡"
    UNKNOWN = "待定"  # 新增：走势未完成，或两端不满足级别L等价条件
```

### flow_relation.py — EdgeFlowInput

```python
@dataclass(frozen=True, slots=True)
class EdgeFlowInput:
    vertex_a: AssetVertex
    vertex_b: AssetVertex
    direction: FlowDirection
    level_id: int | None = None  # 新增

    def __post_init__(self) -> None:
        if self.vertex_a == self.vertex_b:
            raise ValueError(f"自环不合法：{self.vertex_a}")
        # level_id 不在此验证
```

### flow_relation.py — aggregate_vertex_flows

```python
def aggregate_vertex_flows(
    edges: list[EdgeFlowInput],
) -> list[VertexFlowState]:
    if len(edges) == 0:
        raise ValueError("至少需要 1 条边")
    if len(edges) > 6:
        raise ValueError(f"K4 图最多 6 条边，实际 {len(edges)} 条")

    for e in edges:
        if e.direction == FlowDirection.UNKNOWN:
            raise ValueError(
                f"边 {e.vertex_a.value}/{e.vertex_b.value} 方向为 UNKNOWN，"
                "必须在调用前过滤"
            )

    seen: set[frozenset[AssetVertex]] = set()
    for e in edges:
        key = frozenset([e.vertex_a, e.vertex_b])
        if key in seen:
            raise ValueError(f"重复边：{e.vertex_a.value}/{e.vertex_b.value}")
        seen.add(key)

    states: list[VertexFlowState] = []
    for vertex in AssetVertex:
        net = sum(_flow_contribution(e, vertex) for e in edges)
        states.append(VertexFlowState(
            vertex=vertex,
            net_flow=net,
            strength=_classify_resonance(net),
            role=_classify_role(net),
        ))
    return states
```

### flow_relation.py — _flow_contribution

```python
def _flow_contribution(edge: EdgeFlowInput, vertex: AssetVertex) -> int:
    if edge.direction == FlowDirection.UNKNOWN:
        raise ValueError(
            f"UNKNOWN 方向的边不得参与流场聚合："
            f"{edge.vertex_a.value}/{edge.vertex_b.value}"
        )
    if edge.direction == FlowDirection.EQUILIBRIUM:
        return 0
    if edge.direction == FlowDirection.A_TO_B:
        if vertex == edge.vertex_b:
            return +1
        if vertex == edge.vertex_a:
            return -1
        return 0
    # B_TO_A
    if vertex == edge.vertex_a:
        return +1
    if vertex == edge.vertex_b:
        return -1
    return 0
```

### flow_relation.py — _direction_value

```python
def _direction_value(direction: FlowDirection) -> int:
    if direction == FlowDirection.UNKNOWN:
        raise ValueError("UNKNOWN 方向无法映射为数值，调用前请过滤")
    if direction == FlowDirection.A_TO_B:
        return 1
    if direction == FlowDirection.B_TO_A:
        return -1
    return 0  # EQUILIBRIUM
```

### flow_relation.py — disambiguate_cash_signal

```python
def disambiguate_cash_signal(
    edge_inputs: list[EdgeFlowInput],
) -> CashSignalAnalysis:
    """现金边信号消歧：用纯资产子图校验现金边信号的可信度。

    要求恰好 6 条边（完整 K4 图）。部分图缺少现金边或纯资产边时消歧无意义。
    所有边的 direction 必须是已确定状态（不得包含 UNKNOWN）。
    """
    if len(edge_inputs) != 6:
        raise ValueError(
            f"disambiguate_cash_signal 要求恰好 6 条边，实际 {len(edge_inputs)} 条"
        )
    # 其余逻辑不变
    states = aggregate_vertex_flows(edge_inputs)
    ...
```

### flow_relation.py — check_conservation（仅 docstring）

```python
def check_conservation(states: list[VertexFlowState]) -> bool:
    """检查守恒约束：Σnet(V) = 0。

    注意：在部分图（<6条边）上，此函数永远返回 True（有向图拓扑恒等式）。
    守恒破缺的信号意义（数据质量检测）仅在完整 6 条边图上有效。
    """
    return sum(s.net_flow for s in states) == 0
```

---

## 改动文件清单

| 文件 | 改动 |
|------|------|
| `src/newchan/capital_flow.py` | FlowDirection 增加 UNKNOWN |
| `src/newchan/flow_relation.py` | EdgeFlowInput 加 level_id；aggregate_vertex_flows 移除 6 条边约束 + 加上界检查 + 加 UNKNOWN 检查；_flow_contribution 加 UNKNOWN raise；_direction_value 加 UNKNOWN raise；disambiguate_cash_signal 加 6 条边检查；check_conservation docstring 更新 |
| `tests/test_flow_relation.py` | 新增测试用例（见 Codex 方案 §四） |
