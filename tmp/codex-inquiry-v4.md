# Codex 视角：全球资本流转 v4 代码对齐与工程审查

**审查者**: Codex 工位
**日期**: 2026-02-27
**输入**: v4 全文 + 质询上下文 Part1/Part2 + 编排者判断 + 现有代码 + 定义文件

---

## 一、总体评价

v4 是一份质量显著提升的修订版。v3→v4 的十项改动中，大部分精确回应了 v3 质询暴露的问题。编排者的第0条洞察（等价关系级别依赖性）被完整吸收为 §2.5，这是 v4 最有价值的新增内容。

**对齐度评分**: 7.5/10

扣分项：
- 代码层完全没有 v4 新概念的实现（-1.5）
- 定义文件未同步更新（-0.5）
- log-MACD 无代码实现（-0.5）

---

## 二、代码-理论对齐审查

### 2.1 §2.5 级别依赖等价关系 → equivalence.py

**现状**: `validate_pair()` 只检查数据质量（C-1 可比性、C-2 非退化、C-3 流动性），不检查走势完成状态。`EquivalencePair` 没有级别信息。

**v4 要求**: "两个资产在递归层级L上等价，当且仅当两者在层级L上都有已完成的走势类型"

**缺口分析**:

v4 的等价关系有两层：
1. **数据层等价**（现有 C-1/C-2/C-3）：比价K线值不值得看 → 第一层分析的前提
2. **结构层等价**（v4 新增）：两端在同级别有已完成走势 → 第二层（K4关系图）的前提

这两层不是替代关系，是叠加关系。第一层是必要条件（数据质量），第二层是 K4 关系层的额外条件。

**代码改动清单**:

| 优先级 | 改动 | 文件 | 说明 |
|--------|------|------|------|
| P0 | `EquivalencePair` 加 `level_id: int \| None` 字段 | equivalence.py | 标注该等价对在哪个递归层级上成立 |
| P0 | 新增 `validate_structural_equivalence(pair, level_id, move_status_a, move_status_b)` | equivalence.py | 检查两端在指定级别是否都有 settled 走势 |
| P1 | `ValidationResult` 加 `structural_valid: bool \| None` 字段 | equivalence.py | 区分数据层验证和结构层验证的结果 |

**注意**: `validate_structural_equivalence` 的输入需要两端标的在指定级别的走势完成状态。这意味着它依赖 `RecursiveStack` 或等效的递归引擎输出。接口设计应接受走势状态作为参数，不应内部调用递归引擎（关注点分离）。

### 2.2 §4.1 log-MACD → a_macd.py

**现状**: `compute_macd()` 在价格域计算 MACD。没有 log 域版本。

**v4 要求**: 两层口径——价格域用于单边背驰判定，log 域用于跨边力度比较。log 域的关键性质：`MACD(ln A/C) = MACD(ln A/B) + MACD(ln B/C)`。

**数学验证**:

log-MACD 的线性可加性严格成立吗？

```
ln(A/C) = ln(A/B) + ln(B/C)   （对数的加法性，永真）

EMA 是线性算子：EMA(x + y) = EMA(x) + EMA(y)

因此：
  EMA_fast(ln A/C) = EMA_fast(ln A/B) + EMA_fast(ln B/C)
  EMA_slow(ln A/C) = EMA_slow(ln A/B) + EMA_slow(ln B/C)
  MACD_line(ln A/C) = MACD_line(ln A/B) + MACD_line(ln B/C)
  signal(ln A/C) = signal(ln A/B) + signal(ln B/C)   （signal 也是 EMA，线性）
  hist(ln A/C) = hist(ln A/B) + hist(ln B/C)
```

**结论**: log-MACD 的线性可加性严格成立。EMA 的线性性是关键——它对加法封闭。这不是近似，是精确等式。

**代码改动清单**:

| 优先级 | 改动 | 文件 | 说明 |
|--------|------|------|------|
| P0 | 新增 `compute_log_macd(df_raw, ...)` | a_macd.py | 对 `ln(close)` 计算 MACD |
| P1 | 新增 `log_macd_area_for_range(...)` | a_macd.py | log 域的面积计算 |
| P2 | 测试：验证 `log_macd(A/C) ≈ log_macd(A/B) + log_macd(B/C)` | test_macd.py | 可加性回归测试（浮点精度内） |

**实现要点**: `compute_log_macd` 只需在 `compute_macd` 的基础上将 `close` 替换为 `np.log(close)`。但需要处理 `close ≤ 0` 的边界情况（比价K线理论上不会出现负值，但需要防御性检查）。

### 2.3 §4.3 unknown 状态 → capital_flow.py

**现状**: `FlowDirection` 有三个值：`A_TO_B`、`B_TO_A`、`EQUILIBRIUM`。

**v4 要求**: 构建中的边不参与 K4 关系层配置读取。需要一个 `UNKNOWN` 状态表示"该边在此级别上走势未完成"。

**代码改动清单**:

| 优先级 | 改动 | 文件 | 说明 |
|--------|------|------|------|
| P0 | `FlowDirection` 加 `UNKNOWN = "未定"` | capital_flow.py | 走势未完成时的状态 |
| P1 | `flow_relation.py` 的 `_flow_contribution` 处理 UNKNOWN | flow_relation.py | UNKNOWN 边贡献 0 但需标记为"不可靠" |
| P1 | `aggregate_vertex_flows` 返回值加 `n_unknown_edges: int` | flow_relation.py | 让调用方知道有多少边是 unknown |

**关键设计决策**: UNKNOWN 和 EQUILIBRIUM 在 `_flow_contribution` 中都贡献 0，但语义不同：
- EQUILIBRIUM = 走势已确定为盘整（中枢内），这是确定性信息
- UNKNOWN = 走势未完成，方向待定，这是不确定性

下游消费者（配置读取）需要区分这两种 0。建议 `EdgeFlowInput` 加一个 `is_determined: bool` 字段，或者让 `_flow_contribution` 返回 `(int, bool)` 元组。

---

## 三、架构可行性审查

### 3.1 K4 关系层需要哪些新模块？

v4 的两层结构（§2.5）在代码中的映射：

```
第一层（无条件合法）：
  equivalence.py → make_ratio_kline → 缠论管线 → capital_flow.py
  （已有，基本完整）

第二层（有条件合法）：
  需要新增：
  ├── level_scan.py          — 级别扫描：哪些边在指定级别上合法
  ├── k4_configuration.py    — K4 配置读取：27种配置分类 + 消歧
  └── log_macd 扩展          — 跨边力度比较
```

| 新模块 | 优先级 | 输入 | 输出 | 依赖 |
|--------|--------|------|------|------|
| `level_scan.py` | P0 | 6条边 × 各自的 RecursiveStack 状态 | 每条边在每个级别的合法性 + K4 子图 | RecursiveStack, equivalence.py |
| `k4_configuration.py` | P1 | K4 子图 + 各边走势状态 | 配置编号 + 派生边约束范围 + 全局流场读取 | level_scan.py, flow_relation.py |
| `log_macd` (扩展) | P1 | 比价K线 | log 域 MACD | a_macd.py |

### 3.2 级别扫描的实现方案

`level_scan.py` 的核心函数签名：

```python
@dataclass(frozen=True, slots=True)
class EdgeLevelStatus:
    vertex_a: AssetVertex
    vertex_b: AssetVertex
    level_id: int
    is_valid: bool          # 两端在此级别都有 settled 走势
    move_status_a: str      # "settled" / "building" / "none"
    move_status_b: str

def scan_level(
    edges: list[MatrixEdge],
    level_id: int,
    move_states: dict[AssetVertex, MoveStatus],  # 每个顶点在指定级别的走势状态
) -> list[EdgeLevelStatus]:
    """级别扫描：确认哪些边在指定级别上合法。"""
```

**关键问题**: `move_states` 从哪来？每个顶点的走势状态来自该顶点代表标的的 `RecursiveStack`。这意味着级别扫描的前置条件是：四个顶点的代表标的都已经跑过递归引擎。

### 3.3 27种配置的编码方式

27 = 3^3（三条独立边 × 三种状态）。

建议编码方式：

```python
@dataclass(frozen=True, slots=True)
class K4Configuration:
    # 独立边状态（以 CASH 为基准）
    equity_cash: Literal["+", "-", "0"]
    commodity_cash: Literal["+", "-", "0"]
    realestate_cash: Literal["+", "-", "0"]

    @property
    def config_id(self) -> int:
        """0-26 的配置编号。"""
        mapping = {"+": 0, "-": 1, "0": 2}
        return (mapping[self.equity_cash] * 9
                + mapping[self.commodity_cash] * 3
                + mapping[self.realestate_cash])

    @property
    def derived_constraints(self) -> dict[str, list[str]]:
        """派生边的允许状态范围。"""
        # 根据 C1-C3 条件和独立边状态推导
        ...
```

27种配置可以用查找表预计算派生边约束，不需要运行时推导。

---

## 四、定义文件更新审查

### 4.1 dengjia.md 需要加 C-4 吗？

**判断**: 不应加 C-4。

v4 §2.5 的"结构层等价"不是 C-1/C-2/C-3 的第四个条件，而是一个**不同层面的条件**：

- C-1/C-2/C-3 是**数据层**条件：比价K线值不值得看（第一层分析的前提）
- 结构层等价是**K4关系层**条件：两端在同级别有已完成走势（第二层分析的前提）

把它加为 C-4 会混淆两个层面。正确做法：

1. dengjia.md 保持 C-1/C-2/C-3 不变（数据层筛选）
2. 新增一个小节"结构层等价条件"，说明 K4 关系层的额外要求
3. 或者在 liuzhuan.md 中说明 K4 关系层的前置条件（因为这是拓扑层的约束，不是筛选层的）

**建议**: 在 dengjia.md 新增"§ 结构层等价（v4 §2.5）"小节，明确标注这不是 C-4 而是不同层面的条件。同时在 liuzhuan.md 的"前置依赖"中引用此条件。

### 4.2 liuzhuan.md 守恒形式需要改吗？

**现状**: liuzhuan.md 用加法守恒 `Σnet(V) = 0`。v4 §2.3 说"严格的守恒是乘法形式 A/B × B/C × C/A ≡ 1"。

**判断**: 两者是不同层面的约束，不矛盾，不需要改。

- **乘法恒等式** `A/B × B/C × C/A ≡ 1`：价格水平上的代数约束，永真，不携带信息
- **加法守恒** `Σnet(V) = 0`：流场方向上的物理约束，可以被破缺（破缺本身是信号）

v4 §2.3 的措辞"严格的守恒是乘法形式"有误导性——乘法恒等式不是"守恒"，是恒等式（永真命题不携带信息）。liuzhuan.md 的加法守恒才是真正的守恒约束（可以被破缺，破缺有信号意义）。

**建议**: liuzhuan.md 不改。但建议在 v4 §2.3 中明确区分：
- 乘法恒等式 = 代数约束（永真，用于配置分类的搜索空间缩减）
- 加法守恒 = 流场约束（可破缺，破缺是信号）

---

## 五、测试策略

### 5.1 log-MACD 可加性测试

```python
def test_log_macd_additivity():
    """验证 log_macd(A/C) ≈ log_macd(A/B) + log_macd(B/C)"""
    # 构造三个标的的价格序列
    df_a, df_b, df_c = generate_test_prices(n=200)

    ratio_ab = make_ratio_kline(df_a, df_b)
    ratio_bc = make_ratio_kline(df_b, df_c)
    ratio_ac = make_ratio_kline(df_a, df_c)

    macd_ab = compute_log_macd(ratio_ab)
    macd_bc = compute_log_macd(ratio_bc)
    macd_ac = compute_log_macd(ratio_ac)

    # 可加性：hist(A/C) = hist(A/B) + hist(B/C)
    np.testing.assert_allclose(
        macd_ac["hist"].values,
        macd_ab["hist"].values + macd_bc["hist"].values,
        atol=1e-10,
    )
```

### 5.2 K4 配置分类测试

```python
def test_27_configurations_exhaustive():
    """验证 27 种配置的穷尽性和编号唯一性。"""
    seen_ids = set()
    for e in ["+", "-", "0"]:
        for c in ["+", "-", "0"]:
            for r in ["+", "-", "0"]:
                config = K4Configuration(e, c, r)
                assert 0 <= config.config_id <= 26
                assert config.config_id not in seen_ids
                seen_ids.add(config.config_id)
    assert len(seen_ids) == 27

def test_unknown_edge_excluded_from_k4():
    """UNKNOWN 边不参与 K4 配置读取。"""
    edges = make_test_edges(unknown_indices=[0, 1])  # 2条 unknown
    subgraph = scan_level(edges, level_id=2, ...)
    valid_edges = [e for e in subgraph if e.is_valid]
    assert len(valid_edges) == 4  # 6 - 2 = 4
    # 4条边不构成完整 K4，不能做完整配置读取
```

### 5.3 结构层等价测试

```python
def test_structural_equivalence_both_settled():
    """两端都有 settled 走势 → 结构层等价成立。"""
    result = validate_structural_equivalence(
        pair, level_id=2,
        move_status_a="settled",
        move_status_b="settled",
    )
    assert result is True

def test_structural_equivalence_one_building():
    """一端 building → 结构层等价不成立。"""
    result = validate_structural_equivalence(
        pair, level_id=2,
        move_status_a="settled",
        move_status_b="building",
    )
    assert result is False
```

---

## 六、代码改动清单汇总

### P0（阻塞性——v4 核心概念无代码对应）

| # | 改动 | 文件 | 理由 |
|---|------|------|------|
| 1 | `FlowDirection.UNKNOWN` | capital_flow.py | §4.3 构建中边的状态表达 |
| 2 | `compute_log_macd()` | a_macd.py | §4.1 跨边力度比较的基础 |
| 3 | `validate_structural_equivalence()` | equivalence.py | §2.5 结构层等价条件 |
| 4 | `level_scan.py` 新模块 | src/newchan/ | §7.1 级别扫描前置步骤 |

### P1（重要——K4 关系层功能）

| # | 改动 | 文件 | 理由 |
|---|------|------|------|
| 5 | `k4_configuration.py` 新模块 | src/newchan/ | §5 配置分类 + 消歧 |
| 6 | `aggregate_vertex_flows` 支持 UNKNOWN | flow_relation.py | UNKNOWN 边的正确处理 |
| 7 | `log_macd_area_for_range()` | a_macd.py | 跨边力度面积比较 |
| 8 | dengjia.md 加"结构层等价"小节 | .chanlun/definitions/ | 定义文件同步 |

### P2（增强——测试与文档）

| # | 改动 | 文件 | 理由 |
|---|------|------|------|
| 9 | log-MACD 可加性回归测试 | tests/ | 数学性质的工程保证 |
| 10 | 27配置穷尽性测试 | tests/ | 配置分类的正确性 |
| 11 | liuzhuan.md 加"两种守恒"说明 | .chanlun/definitions/ | 消除加法/乘法守恒的混淆 |

---

## 七、与 v4 理论的对齐度细项

| v4 章节 | 理论要点 | 代码对齐度 | 说明 |
|---------|---------|-----------|------|
| §1.1 工程前提 | 时间戳同步、流动性、非零价格 | 8/10 | `validate_pair` 已覆盖大部分，缺缺失值处理策略 |
| §2.1 射影化 | 比价序列在正射影空间 | N/A | 纯数学描述，不需要代码对应 |
| §2.2 独立性约束 | 6边中3条独立 | 5/10 | `flow_relation.py` 有拓扑但无独立/派生边区分 |
| §2.4 方向约束条件化 | C1-C3 充分条件 | 0/10 | 完全无实现 |
| §2.5 两层结构 | 数据层 vs 结构层等价 | 3/10 | 数据层有，结构层无 |
| §4.1 log-MACD | 跨边可加的力度度量 | 0/10 | 完全无实现 |
| §4.3 级别结构 | unknown 状态、递归层级 | 0/10 | FlowDirection 无 UNKNOWN |
| §5 配置约束 | 27种配置分类 | 0/10 | 完全无实现 |
| §7 操作流程 | 级别扫描前置步骤 | 0/10 | 完全无实现 |

---

## 八、关键风险与建议

### 风险1：结构层等价依赖递归引擎的全量运行

级别扫描需要四个顶点的代表标的都跑过 `RecursiveStack`。这意味着 K4 关系层的实时运行需要同时维护 4 个（或更多）标的的递归状态。工程复杂度不低。

**建议**: 先实现离线/批处理版本（历史数据回测），再考虑实时版本。

### 风险2：v4 §2.3 的"乘法守恒"措辞可能误导下游

v4 说"严格的守恒是乘法形式"，但乘法恒等式是永真的（不携带信息），真正有信号意义的守恒是 liuzhuan.md 的加法形式。如果下游代码按"乘法守恒"实现，会得到一个永远返回 True 的函数——无用。

**建议**: v4 修正措辞，或在代码注释中明确区分。

### 风险3：UNKNOWN 和 EQUILIBRIUM 的语义混淆

两者在 `_flow_contribution` 中都贡献 0，但含义完全不同。如果不在类型层面区分，下游消费者无法判断"net=0"是"确定的均衡"还是"信息不足"。

**建议**: `EdgeFlowInput` 加 `is_determined: bool` 字段，或 `_flow_contribution` 返回 `Optional[int]`（None = unknown）。

---

## 九、总体评价

v4 在理论层面是一份高质量的修订。编排者的第0条洞察（等价关系级别依赖性）被完整吸收，两层结构的划分清晰，"完全分类定理"降级为"配置约束命题"是正确的。log-MACD 的数学基础严格成立。

**主要问题在代码层**: v4 的核心新概念（结构层等价、级别扫描、UNKNOWN 状态、log-MACD、K4 配置分类）在代码中完全没有对应。现有代码只覆盖了 v4 第一层（单条比价线分析），第二层（K4 关系图）的代码基础设施几乎为零。

**优先行动**: P0 的四项改动（UNKNOWN 状态、log-MACD、结构层等价验证、级别扫描模块）是 v4 落地的最小可行集。建议按此顺序实施。
