# Codex 视角质询报告：全球资本流转 v4（代码对齐与工程审查）

## 总体评价

v4 的理论改动方向正确，但与现有代码的对齐度低。核心问题：v4 引入了三个新概念（级别依赖等价、两层MACD、K4浮现性），现有代码对这三个概念的支持为零。需要新建模块而非修改现有模块。好消息是现有代码不需要大改——它们正确实现了第一层，v4 的改动主要影响尚未实现的第二层。

---

## 1. 代码-理论对齐审查

### 1.1 级别依赖等价关系（v4 §2.5）vs equivalence.py

**现状**：`validate_pair()` 检查 C-1/C-2/C-3（数据质量），完全不知道递归层级。

**v4 要求**：两个资产在递归层级L上等价 ⟺ 两者在L上都有已完成走势。

**分析**：这不是对 `validate_pair` 的修改，而是一个**新的、独立的判定函数**。`validate_pair` 回答"这对比价K线值不值得看"（数据层），新函数回答"这对比价在第L层递归上是否可用于K4关系"（结构层）。

**代码方案**：
```python
# 新函数，不修改 validate_pair
def check_level_equivalence(
    moves_a: list[Move],  # 资产A的递归走势列表
    moves_b: list[Move],  # 资产B的递归走势列表
    target_level: int,
) -> bool:
    """检查两个资产在目标递归层级上是否都有已完成走势。"""
    a_settled = any(m.level_id == target_level and m.settled for m in moves_a)
    b_settled = any(m.level_id == target_level and m.settled for m in moves_b)
    return a_settled and b_settled
```

**优先级**：P1（第二层的前置条件，但第一层不需要）
**改动量**：新增函数，不改现有代码。依赖 RecursiveStack 的输出。

### 1.2 log-MACD（v4 §4.1）vs a_macd.py

**现状**：`compute_macd()` 接受 DataFrame，对 close 列计算 EMA 差值。价格域。

**v4 要求**：第二层跨边力度比较用 log-MACD。

**代码方案**：
```python
def compute_log_macd(df: pd.DataFrame, **kwargs) -> pd.DataFrame:
    """对 log(close) 计算 MACD。"""
    log_df = df.copy()
    log_df["close"] = np.log(df["close"])
    return compute_macd(log_df, **kwargs)
```

**可加性验证测试**：
```python
def test_log_macd_additivity():
    """MACD(ln A/C) ≈ MACD(ln A/B) + MACD(ln B/C)"""
    macd_ac = compute_log_macd(ratio_kline_ac)
    macd_ab = compute_log_macd(ratio_kline_ab)
    macd_bc = compute_log_macd(ratio_kline_bc)
    # 暂态期后应近似相等
    np.testing.assert_allclose(
        macd_ac["hist"].iloc[60:],
        macd_ab["hist"].iloc[60:] + macd_bc["hist"].iloc[60:],
        atol=1e-10,
    )
```

**优先级**：P1（第二层消歧需要）
**改动量**：新增一个薄包装函数 + 测试。不改 compute_macd。

### 1.3 unknown 状态（v4 §4.3）vs FlowDirection

**现状**：`FlowDirection` 有三个值：A_TO_B, B_TO_A, EQUILIBRIUM。

**v4 要求**：构建中的边标记 unknown。

**代码方案**：
```python
class FlowDirection(Enum):
    A_TO_B = "A→B"
    B_TO_A = "B→A"
    EQUILIBRIUM = "均衡"
    UNKNOWN = "unknown"  # 走势未完成，方向不可判定
```

**影响分析**：`strokes_to_flows()` 不受影响（它处理已有笔，不产出 UNKNOWN）。UNKNOWN 只在 K4 关系层的级别扫描中使用——当某条边在目标层级上没有已完成走势时，标记为 UNKNOWN。

**优先级**：P1
**改动量**：FlowDirection 加一个枚举值 + K4 关系层使用。现有测试不受影响。

---

## 2. 架构可行性

### 2.1 K4 关系层需要的新模块

| 模块 | 职责 | 依赖 | 优先级 |
|------|------|------|--------|
| `k4_graph.py` | K4 图结构：6条边、4个顶点、子图判定 | matrix_topology.py | P0 |
| `k4_level_scan.py` | 级别扫描：在目标层级上判定哪些边合法 | RecursiveStack, check_level_equivalence | P0 |
| `k4_configuration.py` | 27种配置分类 + 派生边允许范围表 | k4_graph.py | P1 |
| `log_macd.py` | log-MACD 计算 | a_macd.py | P1 |
| `k4_disambiguator.py` | 消歧：观测派生边 + 一致性检验 | k4_configuration.py, log_macd.py | P2 |

### 2.2 级别扫描实现

```python
@dataclass(frozen=True)
class K4LevelScan:
    """某递归层级上的 K4 子图扫描结果。"""
    target_level: int
    active_edges: frozenset[tuple[str, str]]  # 合法边（两端都有已完成走势）
    unknown_edges: frozenset[tuple[str, str]]  # 不合法边
    subgraph_type: str  # "complete"(6), "5-edge", "4-edge", "triangle"(3), "sparse"(<3)
```

需要的数据：每个资产的 RecursiveStack 输出（各层级的 settled moves）。

### 2.3 27种配置编码

**查表**，不是运行时计算。27种配置是静态的，派生边允许范围也是静态的。

```python
# 配置表：(E/$, C/$, R/$) → {E/C允许范围, E/R允许范围, C/R允许范围}
CONFIG_TABLE: dict[tuple[int,int,int], dict[str, set[int]]] = {
    (+1,+1,+1): {"E/C": {+1,-1,0}, "E/R": {+1,-1,0}, "C/R": {+1,-1,0}},  # 无约束
    (+1,-1,-1): {"E/C": {+1}, "E/R": {+1}, "C/R": {+1,-1,0}},  # 强约束
    # ... 27种全部穷举
}
```

**优先级**：P1（这是约束命题的核心产出）

---

## 3. 定义文件更新需求

### 3.1 dengjia.md

需要加入 C-4（结构完成条件）：

```
C-4 结构完成（级别依赖）：两个标的在目标递归层级L上都有已完成的走势类型（Move.settled=True）。
C-4 是 K4 关系层的前置条件，不影响第一层（单边比价分析）。
```

C-4 与 C-1/C-2/C-3 不在同一层面：C-1/C-2/C-3 是数据层筛选（一次性判定），C-4 是结构层判定（随时间变化）。

### 3.2 liuzhuan.md

守恒形式不需要改。两种守恒并存：
- 乘法恒等式（价格水平）：A/B × B/C × C/A ≡ 1（永真）
- 加法守恒（流场方向）：Σ net(V) = 0（精确）

需要加一段说明两者的关系。

### 3.3 新定义文件

需要新建 `k4_configuration.md`：
- K4 配置约束命题的形式化定义
- 27种配置表（含派生边允许范围）
- 两层结构的定义
- K4 浮现性条件

---

## 4. v3 遗留问题的代码影响

### 4.1 模式D/E "需观测"

不需要扩展 FlowDirection。"需观测"不是一个状态——它是说配置表中该位置的允许范围是 {+,–,0}，需要实际观测来确定。代码中用 `CONFIG_TABLE` 的允许范围集合大小来判断：
- 集合大小=1：强约束，不需要观测
- 集合大小>1：需要观测消歧

### 4.2 K4 浮现性

```python
def k4_emergence_level(stacks: dict[str, RecursiveStack]) -> int | None:
    """找到 K4 完整浮现的最低递归层级。返回 None 表示无完整 K4。"""
    for level in range(1, max_level + 1):
        scan = level_scan(stacks, level)
        if scan.subgraph_type == "complete":
            return level
    return None
```

---

## 5. 测试策略

### 5.1 新增测试

| 测试 | 覆盖 | 优先级 |
|------|------|--------|
| test_log_macd_additivity | log-MACD 可加性（暂态期后） | P1 |
| test_level_equivalence | check_level_equivalence 正确性 | P1 |
| test_config_table_completeness | 27种配置全部覆盖 | P1 |
| test_config_table_correctness | 每种配置的允许范围数学正确 | P1 |
| test_k4_level_scan | 级别扫描子图判定 | P1 |
| test_k4_emergence | K4 浮现性判定 | P2 |
| test_disambiguator | 消歧逻辑 | P2 |

### 5.2 现有测试影响

- test_equivalence.py：不受影响（C-1/C-2/C-3 不变）
- test_capital_flow.py：不受影响（第一层不变）
- FlowDirection 加 UNKNOWN：现有测试不使用 UNKNOWN，不受影响

---

## 6. 对齐度评分

| v4 理论改动 | 现有代码基础 | 对齐度 | 工作量 |
|------------|------------|--------|--------|
| §2.4 C1-C3 条件化 | RecursiveStack 有 settled 标记 | 60% | 低 |
| §2.5 两层结构 | 第一层完整，第二层为零 | 30% | 高 |
| §2.5 级别依赖等价 | RecursiveStack 输出可用 | 40% | 中 |
| §4.1 log-MACD | compute_macd 可复用 | 70% | 低 |
| §4.3 unknown 机制 | FlowDirection 可扩展 | 80% | 低 |
| §5 配置约束命题 | matrix_topology 有 K4 骨架 | 20% | 高 |
| §7 级别扫描 | RecursiveStack 可查询 | 40% | 中 |

**总体对齐度**：约 40%。第一层代码完整且不需要改动。第二层需要从零开始，但可以复用 RecursiveStack 和 matrix_topology 的基础设施。

**关键阻塞点**：27种配置的派生边允许范围表——这是理论和代码的共同阻塞点。v4 没有给出这个表，代码也无法实现。必须先在理论层完成穷举，才能编码。
