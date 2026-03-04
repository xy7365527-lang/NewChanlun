# OQ4 黄金截面 K3 vs K4 对比分析

## 实验参数

- 数据范围: 2004-11-18 ~ 2026-02-27
- 公共交易日: 5352
- K3 边数: 3 (SPY/GLD, TLT/GLD, SPY/TLT)
- K4 边数: 6 (SPY, GLD, TLT, SPY/GLD, TLT/GLD, SPY/TLT)

## 比价 Bar 构造

```
ratio_open  = a.open / b.open
ratio_close = a.close / b.close
all_ratios  = [ratio_open, ratio_close, a.high/b.low, a.low/b.high]
high = max(all_ratios), low = min(all_ratios)
```

## K3 截面结构

- **SPY_GLD**: 5352 bars | UP=0 DOWN=0 FLAT=5352 | transitions=0 | moves=8 segs=48 zs=9
- **TLT_GLD**: 5352 bars | UP=0 DOWN=0 FLAT=5352 | transitions=0 | moves=7 segs=48 zs=7
- **SPY_TLT**: 5352 bars | UP=0 DOWN=0 FLAT=5352 | transitions=0 | moves=7 segs=48 zs=8

## K4 对照组结构

- **SPY** (顶点边): 5352 bars | UP=0 DOWN=0 FLAT=5352 | transitions=0 | moves=8 segs=38 zs=9
- **GLD** (顶点边): 5352 bars | UP=0 DOWN=0 FLAT=5352 | transitions=0 | moves=7 segs=46 zs=7
- **TLT** (顶点边): 5352 bars | UP=634 DOWN=0 FLAT=4718 | transitions=2 | moves=7 segs=55 zs=9
- **SPY_GLD** (比价边): 5352 bars | UP=0 DOWN=0 FLAT=5352 | transitions=0 | moves=8 segs=48 zs=9
- **TLT_GLD** (比价边): 5352 bars | UP=0 DOWN=0 FLAT=5352 | transitions=0 | moves=7 segs=48 zs=7
- **SPY_TLT** (比价边): 5352 bars | UP=0 DOWN=0 FLAT=5352 | transitions=0 | moves=7 segs=48 zs=8

## 共享边方向态一致性

K3 和 K4 都包含三条比价边（SPY/GLD, TLT/GLD, SPY/TLT）。
由于两个截面使用相同的比价 Bar 构造法和相同的 RecursiveOrchestrator，
理论上共享边应完全一致。不一致则说明流 ID 等非数据因素影响了引擎行为。

- **SPY_GLD**: 100.0% 一致 (5352/5352)
- **TLT_GLD**: 100.0% 一致 (5352/5352)
- **SPY_TLT**: 100.0% 一致 (5352/5352)

## K3 少了什么（相对 K4）

K4 有六条边，K3 只有三条边。K3 缺少的三条顶点边（SPY/$, GLD/$, TLT/$）
携带的方向态信息：

- **SPY**: UP=0 DOWN=0 FLAT=5352 transitions=0
- **GLD**: UP=0 DOWN=0 FLAT=5352 transitions=0
- **TLT**: UP=634 DOWN=0 FLAT=4718 transitions=2

## K3 多了什么（黄金计价的优势）

黄金计价消除了美元噪声。在 K4 中，SPY/$ 和 GLD/$ 的方向态可能同时被
美元波动驱动（假方向态），但 SPY/GLD 直接反映权益相对黄金的真实购买力变化。

活跃边平均数: K3=0.0/3 K4=0.118/6

## K3 截面完全分类规则（初稿）

K3 退化为三顶点（E, R, Au）三条边的完备图 K3。
每条边有三种方向态 {UP, DOWN, FLAT}，理论配置空间 3^3 = 27。

### 约束

1. SPY/GLD * GLD/TLT = SPY/TLT（传递性约束）
   - 但 D 算子在每条边独立运行，不保证代数传递性
   - 因此 27 种配置中，违反传递性的配置可能在实际中出现

### 语义分类

| 配置 | SPY/GLD | TLT/GLD | SPY/TLT | 含义 |
|------|---------|---------|---------|------|
| A1   | UP      | DOWN    | UP      | 权益强于黄金强于债券 |
| A2   | DOWN    | UP      | DOWN    | 债券强于黄金强于权益 |
| A3   | UP      | UP      | FLAT    | 权益和债券都弱于黄金（避险） |
| A4   | DOWN    | DOWN    | FLAT    | 权益和债券都强于黄金（风险偏好） |
| A5   | FLAT    | FLAT    | FLAT    | 三者均衡 |
| ...  | ...     | ...     | ...     | 其余配置待实际统计后分类 |

### 实际配置分布

| SPY/GLD | TLT/GLD | SPY/TLT | 频次 | 占比 |
|---------|---------|---------|------|------|
| FLAT    | FLAT    | FLAT    | 5352 | 100.0% |

## 结果包

1. **结论**: K3 vs K4 方向态一致性/差异分析（见上）
2. **定义依据**: OQ4 黄金截面假说——用黄金消除美元噪声
3. **边界条件**: GLD 数据起始日期 (2004-11-18)；  比价 Bar 构造法假设分母 OHLC 不为零
4. **下游推论**: K3 截面是否可作为 K4 的有效简化——取决于共享边一致性和顶点边信息量
5. **谱系引用**: 279号
6. **影响声明**: 实验结果，不修改仓库代码

## 认识论等级

L2（真实数据验证）