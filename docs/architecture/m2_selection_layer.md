# M2 选股正则化层

## 位置

M2 是路线图五个里程碑中的第二个（M1 回测验证 → **M2 选股正则化** → M3 全市场实时信号 → M4 风控与执行 → M5 生产加固）。

M2 的输入：M1 回测发现（指数做空无 alpha）+ 卢麒元 ω 理论 + K4 拓扑。
M2 的输出：品种池 + 方向表 → 供 M1 回测脚本和 M3 实时管线消费。

## 架构

```
┌─────────────────────────────────────────────────────────┐
│                    M2 选股层                             │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ asset_       │  │ omega_       │  │ config_      │  │
│  │ classifier   │  │ regime       │  │ space        │  │
│  │              │  │              │  │ (K4极性)     │  │
│  │ symbol →     │  │ ω_series →   │  │ config →     │  │
│  │ AssetProfile │  │ OmegaState   │  │ polarity     │  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │
│         │                 │                 │          │
│         └────────┬────────┴────────┬────────┘          │
│                  │                 │                    │
│           ┌──────▼─────────────────▼──────┐            │
│           │   selection_pool              │            │
│           │   apply_regime_adjustment()   │            │
│           │   build_selection_pool()      │            │
│           └──────────────┬────────────────┘            │
│                          │                             │
│                  ┌───────▼───────┐                     │
│                  │ SelectionPool │                     │
│                  │ + 方向表       │                     │
│                  └───────┬───────┘                     │
│                          │                             │
│               pool_to_backtest_config()                │
│                          │                             │
└──────────────────────────┼─────────────────────────────┘
                           │
                           ▼
                    M1 回测 / M3 实时
```

## 模块职责

### asset_classifier.py

静态分类器。从标的代码推断资产类型和基础方向策略。

| 资产类型 | 方向策略 | 依据 |
|---------|---------|------|
| ETF/指数 | long_only | M1 回测验证：指数做空无 alpha |
| 个股 | both | 个股可多可空 |
| 期货 | both | 期货天然多空 |

**long_only 是硬约束**——regime 和 K4 极性都不改变此方向。这是 M1 的 L2 验证结果。

### omega_regime.py

ω = 金价/油价。ω 的趋势反映美元信用周期。

| regime | ω 趋势 | 含义 | 利好 |
|--------|--------|------|------|
| BULL_EQUITY | ω 下降 | 信用扩张 | 股票 |
| BULL_COMMODITY | ω 上升 | 信用收缩 | 大宗 |
| NEUTRAL | ω 震荡 | 方向不明 | — |

检测方法：双均线偏离（短20/长60），偏离率 > 2% 判定方向。

两个入口：
- `detect_regime(omega_series)` — 从 ω 时间序列检测（统计方法）
- `regime_from_walk_direction(direction)` — 从缠论 D-operator 的走势方向推断（接入比价管线时使用）

### selection_pool.py

三信号合流。核心函数 `apply_regime_adjustment`:

1. **基础方向** ← asset_classifier（硬约束层）
2. **置信度调整** ← omega_regime × asset_type（顺 regime → HIGH，逆 regime → LOW）
3. **共振修正** ← K4 polarity（极性与 regime 一致 → 维持 HIGH，冲突 → 降级）

输出 `SelectionPool`：按置信度降序排列的品种条目列表。

## regime × polarity 决策矩阵

regime = BULL_EQUITY（信用扩张）:

| polarity | 指数 | 个股 | 期货 |
|----------|------|------|------|
| > 0 (risk-on) | long_only/HIGH | both/HIGH | both/LOW |
| = 0 | long_only/HIGH | both/HIGH | both/LOW |
| < 0 (risk-off) | long_only/MEDIUM | both/MEDIUM | both/LOW |

regime = BULL_COMMODITY（信用收缩）:

| polarity | 指数 | 个股 | 期货 |
|----------|------|------|------|
| > 0 (risk-on) | long_only/LOW | both/LOW | both/HIGH |
| = 0 | long_only/LOW | both/LOW | both/HIGH |
| < 0 (risk-off) | long_only/LOW | both/LOW | both/HIGH |

regime = NEUTRAL:

| polarity | 指数 | 个股 | 期货 |
|----------|------|------|------|
| 任意 | long_only/MEDIUM | both/MEDIUM | both/MEDIUM |

## M1 对接接口

```python
from newchan.strategy.selection_pool import build_selection_pool, pool_to_backtest_config
from newchan.strategy.omega_regime import OmegaRegime

pool = build_selection_pool(
    symbols=("QQQ", "OKLO", "NYMEX:CL1!"),
    regime=OmegaRegime.BULL_EQUITY,
    k4_polarity=2,
)

config = pool_to_backtest_config(pool)
# → {"QQQ": False, "OKLO": True, "NYMEX:CL1!": True}
#    False = short_enabled=False (long_only)
#    True  = short_enabled=True  (both)
```

回测脚本通过 `config[symbol]` 决定是否启用做空逻辑。

## 更新频率

品种池在 **regime 变化时**更新，不是每 bar 更新。regime 判定基于均线偏离，变化频率远低于 bar 频率。K4 配置变化同样是低频事件（配置空间的邻居转换需要走势完成）。

## 与帕萨卡利亚品种池的关系

`scanner_pool.py`（帕萨卡利亚条件设定链）是更精细的选股层，使用折叠等价类和商空间排序。M2 的 `selection_pool.py` 是宏观层面的方向过滤器，可作为帕萨卡利亚的**上游前置**——先用 regime + K4 确定大方向和置信度，再用帕萨卡利亚精选具体标的。

```
M2 selection_pool → 方向过滤 → scanner_pool → 折叠等价类排序 → 代表元
```

## 认识论标注

| 组件 | 等级 | 依据 |
|------|------|------|
| 资产分类规则 | L2 | M1 回测验证：指数做空无 alpha |
| ω regime 定义 | L2 | 2025-02-20 regime 断点在真实数据中确认 |
| 双均线参数 (20/60/0.02) | L1 | 合成数据管线验证，真实数据优化待做 |
| K4 极性共振权重 | L0 | 从定义推导，尚需 L2 验证 |
| regime × polarity 矩阵 | L0 | 组合逻辑，尚需 L2 验证实际收益差异 |

## 文件清单

| 文件 | 职责 |
|------|------|
| `src/newchan/strategy/asset_classifier.py` | 标的分类 |
| `src/newchan/strategy/omega_regime.py` | ω regime 检测 |
| `src/newchan/strategy/selection_pool.py` | 品种池合流 |
| `analysis/test_selection_pool.py` | L1 验证脚本 |
| `analysis/m2_backtest_integration.py` | M1 回测对接示例 |

## 谱系引用

- M1 回测发现：指数做空无 alpha（选股层设计的直接动因）
- 卢麒元框架：L=M×ω 与 GDP 协整，ω 是资本三流的温度计
- 用户交易方向：押注金油比下降（油涨），超大级别周线线段一买
- 352号：多标的扫描器设计（帕萨卡利亚品种池）
- 292号：折叠拓扑本体论
