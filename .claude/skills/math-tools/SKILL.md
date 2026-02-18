---
name: math-tools
description: >
  缠论形式化的数学工具对照表。提供缠论语言↔数学语言的映射、引入位置约束、验证模板。
  不默认加载——只在等价关系封闭后、不变量读出前激活。
activation_condition: "等价关系（dengjia 定义）已封闭（v1.x 已结算）且不变量尚未被读出（无 invariant 定义）"
activation_check: "definitions/dengjia.md 状态=已结算 AND definitions/invariant.md 不存在"
genealogy_source: "026-math-tools-crystallization"
source_pdfs:
  - docs/pdfs/等价关系与不变量 - 数学工具应用分析.pdf
  - docs/pdfs/等价关系与不变量 - 新颖点：保持尺度态射而非仅等价.pdf
  - docs/pdfs/等价关系与不变量 - 候选不变量：行为同余商.pdf
  - docs/pdfs/等价关系与不变量 - 桥接：新缠行为等价与类型空间.pdf
  - docs/pdfs/等价关系与不变量 - 启动等价关系：先从分解入手.pdf
  - docs/pdfs/写作 - New‑Chan 不变量映射：断点检测与最小验证框架.pdf
---

# 数学工具：缠论形式化对照表

> **此 skill 是压缩物，不是 PDF 的副本。** 只含对照表、约束、验证模板。
> 详细推导和证明留在源 PDF 中，需要时按需加载。

## §1 引入位置约束

数学工具的引入有严格的阶段门控：

```
L1 等价关系封闭
       ↓
  ┌─────────────┐
  │ 数学工具激活 │ ← 本 skill 的激活窗口
  └─────────────┘
       ↓
L2 不变量读出
```

**三个前提条件**（缺一不可）：
1. 等价关系不可新增 — 对照表的锚点必须稳定
2. 等价关系不可回滚 — 已封闭的定义不再开放
3. 争议只发生在 L2（不变量的选取），不回溯 L1

**过早引入的风险**：等价关系未封闭时，数学语言映射的锚点不稳定，引入范畴论语言会过早固化尚在变动的概念。

## §2 核心对照表

### L0 — 合法操作 ↔ 态射

| 缠论语言 | 数学语言 | 说明 |
|----------|---------|------|
| 合法否定操作 | 态射（morphism） | 允许的变换/转换 |
| 分型→笔→线段 的递归构造 | 函子（functor） | 保结构映射 |
| 走势类型的递归完备性 | 范畴的闭合性 | 态射的复合仍在范畴内 |

### L1 — 不可区分 ↔ 等价关系

| 缠论语言 | 数学语言 | 说明 |
|----------|---------|------|
| "从走势结构看不出差别" | 行为等价（behavioral equivalence） | 观测不可区分 |
| C-1/C-2/C-3 筛选条件 | 等价关系的预筛（不是定义） | 见 025 号谱系 |
| 比价走势的结构同质性 | Obs-bisimulation | 基于可观测行为的双模拟 |

### L2 — 跨级别保持 ↔ 自然变换

| 缠论语言 | 数学语言 | 说明 |
|----------|---------|------|
| 级别递归保持不变的性质 | 自然变换（natural transformation） | 函子之间的结构映射 |
| "先变级别再算不变量 = 先算不变量再变级别" | 换序律 I∘f = f'∘I | 自然变换的交换图 |
| 最小可操作结构类 | 行为商（behavioral quotient） | Myhill-Nerode 商 |

### L3 — 不变量 ↔ 可下注物

| 缠论语言 | 数学语言 | 说明 |
|----------|---------|------|
| 跨标的不变的量 | 不变量（invariant） | 等价类上的良定义函数 |
| 可下注 = 可操作的区分 | 不变量映射 I: X → V | 从走势空间到值空间 |
| "这个信号在换标的后还成立吗？" | 迁移不变性 | I(x) = I(φ(x)) |

## §3 关键数学结构

### 3.1 换序律（自然变换的核心性质）

```
走势空间_级别n  ──f──→  走势空间_级别(n+1)
      │                        │
      I                        I'
      │                        │
      ↓                        ↓
   值空间_n    ──f'──→    值空间_(n+1)

换序律：I' ∘ f = f' ∘ I
```

含义：**先升级别再算不变量** 和 **先算不变量再升级别** 得到相同结果。
如果某个量满足换序律 → 它是真正的跨级别不变量。
如果违反换序律 → 它是级别依赖的量，不能作为跨级别操作依据。

### 3.2 行为商（Myhill-Nerode 定理的对偶）

```
走势全集 ──π──→ 行为商（最小可操作结构类）
```

行为商 = 将"从交易角度不可区分的走势"合并为同一类。这是最粗的等价关系使得所有交易决策仍然良定义。

- **Coalgebra 视角**：走势是状态，后续演化是转移函数，行为等价 = 后续演化不可区分
- **与缠论的对应**：走势类型的分类就是行为商的具体实例——同级别的走势被分为"上涨/下跌/盘整"三类，因为从操作角度它们的后续演化空间不同

### 3.3 Obs-bisimulation（可观测双模拟）

比 bisimulation 更精细：不要求所有状态转移都匹配，只要求**可观测的**状态转移匹配。

与缠论的对应：两条比价走势不需要K线级别完全同步，只需要在**缠论可辨认的结构层面**（笔、线段、中枢）表现出相同模式。

## §4 验证模板

### 4.1 换序律测试

```python
def test_commuting_diagram(pair_A, pair_B, level_n, level_n1, invariant_fn):
    """
    验证不变量 invariant_fn 是否满足换序律。

    pair_A, pair_B: 两个比价系统的K线数据
    level_n, level_n1: 两个相邻级别
    invariant_fn: 待验证的不变量函数

    换序律: I(upgrade(x)) == upgrade'(I(x))
    """
    # 路径1: 先升级别，再算不变量
    structure_n = pipeline(pair_A, level=level_n)
    structure_n1 = pipeline(pair_A, level=level_n1)  # 升级别
    inv_path1 = invariant_fn(structure_n1)

    # 路径2: 先算不变量，再看级别变换后的值
    inv_n = invariant_fn(structure_n)
    inv_path2 = level_transform(inv_n, level_n, level_n1)

    # 换序律检验
    assert inv_path1 == inv_path2, (
        f"换序律违反: path1={inv_path1}, path2={inv_path2}"
    )
```

### 4.2 跨标的迁移不变性测试

```python
def test_cross_instrument_invariance(pairs, invariant_fn, tolerance=0.05):
    """
    验证不变量在等价类内的标的间是否保持。

    pairs: 已通过 C-1/C-2/C-3 筛选的比价系统列表
    invariant_fn: 待验证的不变量函数
    tolerance: 允许的偏差（结构量，非绝对值）
    """
    invariants = {}
    for pair in pairs:
        structure = pipeline(pair)
        invariants[pair.name] = invariant_fn(structure)

    # 等价类内的不变量应该一致
    values = list(invariants.values())
    for i, v1 in enumerate(values):
        for j, v2 in enumerate(values):
            if i < j:
                deviation = structural_distance(v1, v2)
                assert deviation <= tolerance, (
                    f"迁移不变性违反: {pairs[i].name} vs {pairs[j].name}, "
                    f"deviation={deviation} > tolerance={tolerance}"
                )
```

## §5 按需加载指引

本 skill 是入口压缩物。需要深入某个方向时，加载对应 PDF：

| 需求 | 加载 |
|------|------|
| L0-L5 分层框架详细推导 | 数学工具应用分析.pdf |
| 换序律的严格定义和证明 | 新颖点：保持尺度态射.pdf |
| 行为商/Myhill-Nerode 的数学细节 | 候选不变量：行为同余商.pdf |
| Obs-bisimulation 与 σ-algebra | 桥接：新缠行为等价与类型空间.pdf |
| 结构本体枚举（大量具体例子） | 启动等价关系.pdf（406页，按章节加载） |
| 断点检测/最小验证框架实现 | 不变量映射.pdf（139页，按章节加载） |
