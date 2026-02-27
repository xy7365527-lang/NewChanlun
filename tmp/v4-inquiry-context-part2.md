# 全球资本流转 v4 双向质询上下文（Part 2: 代码与定义原文）

## equivalence.py 完整代码

```python
"""等价对管理 — 比价关系的形式化基础。

概念溯源：
  [旧缠论] 第9课 — "比价关系的变动，也可以构成一个买卖系统，
    这个买卖系统是和市场资金的流向相关的"
  [旧缠论] 第72/73课 — 比价关系为三个独立系统之一
  [新缠论] 等价关系严格定义 — ratio_relation_v1.md §2
"""

from __future__ import annotations
from dataclasses import dataclass
import numpy as np
import pandas as pd

@dataclass(frozen=True, slots=True)
class EquivalencePair:
    sym_a: str
    sym_b: str
    category: str = ""
    @property
    def label(self) -> str:
        return f"{self.sym_a}/{self.sym_b}"

@dataclass(frozen=True, slots=True)
class ValidationResult:
    valid: bool
    reason: str = ""
    cv: float | None = None
    stroke_mean_pct: float | None = None
    macd_norm_hist: float | None = None
    n_strokes: int | None = None

MIN_OVERLAP = 5
_MIN_BARS_FOR_PIPELINE = 30
_T_CV = 0.01
_T_STROKE_PCT = 0.005
_T_DYNAMICS_PCT = 0.0001
_T_LIQUIDITY = 0.5

def validate_pair(df_a, df_b, *, min_overlap=MIN_OVERLAP,
                  t_cv=_T_CV, t_stroke_pct=_T_STROKE_PCT,
                  t_dynamics_pct=_T_DYNAMICS_PCT, t_liquidity=_T_LIQUIDITY):
    """验证两个标的是否满足等价对条件。
    C-1 可比性, C-2 非退化（三层退化检测）, C-3 流动性"""
    # C-3 流动性 → C-1 可比性 → C-2 非退化（三层）
    ...

def make_ratio_kline(df_a, df_b):
    """构造比价K线：A/B。OHLC 四列分别除法，volume 取 A。"""
    idx = df_a.index.intersection(df_b.index)
    a, b = df_a.loc[idx], df_b.loc[idx]
    result = pd.DataFrame(index=a.index)
    result["open"] = a["open"] / b["open"]
    result["high"] = a["high"] / b["high"]
    result["low"] = a["low"] / b["low"]
    result["close"] = a["close"] / b["close"]
    if "volume" in a.columns:
        result["volume"] = a["volume"]
    return result
```

## capital_flow.py 完整代码

```python
"""比价走势到资本流转方向推断。"""

from __future__ import annotations
from dataclasses import dataclass
from enum import Enum
from newchan.a_stroke import Stroke
from newchan.equivalence import EquivalencePair

class FlowDirection(Enum):
    A_TO_B = "A→B"
    B_TO_A = "B→A"
    EQUILIBRIUM = "均衡"

@dataclass(frozen=True, slots=True)
class StrokeFlow:
    stroke_index: int
    direction: FlowDirection
    sym_from: str
    sym_to: str
    magnitude: float

def strokes_to_flows(pair: EquivalencePair, strokes: list[Stroke]) -> list[StrokeFlow]:
    """将比价笔序列映射为资本流转序列。"""
    return [_map_stroke(pair, stroke, index) for index, stroke in enumerate(strokes)]
```

## 级别递归核心定义（level_recursion.md 摘要）

```
级别 = 递归层级 level_id : ℕ（从0开始）

Move[0]    = Segment（线段，归纳基底）
Center[k]  = 三个连续 Move[k-1] 的价格区间重叠部分
Move[k]    = 包含 Center[k] 的走势类型实例

递归终止：len(Move[k]) < 3 时停止

口径A（递归级别）是唯一正式路径。
"日线级别"不是"用日线K线"，是"递归构造到达了某个层次"。
```

## 流转关系核心定义（liuzhuan.md 摘要）

```
四矩阵 K4：4顶点（EQUITY/COMMODITY/REALESTATE/CASH），6条边
每条边 = 一条比价K线上的缠论走势

顶点流量：net(V) = Σ flow(eᵢ, V)，i=1..3
  flow(e,V) = +1（流入V）/ -1（流出V）/ 0（均衡）

共振：|net(V)| ≥ 2
守恒：Σ net(V) = 0（加法形式）

注意：v4 说严格守恒是乘法形式 A/B × B/C × C/A ≡ 1
这与 liuzhuan.md 的加法守恒 Σnet=0 是不同层面的约束：
- 乘法恒等式：价格水平上的代数约束（永真）
- 加法守恒：流场方向上的物理约束（资本不凭空产生）
两者是否等价？是否互补？这是质询点之一。
```

## 等价关系核心定义（dengjia.md 摘要）

```
等价对条件：
  C-1 可比性：重叠交易时间窗口
  C-2 非退化：A/B 比价不是常数
  C-3 流动性：充足市场深度

数学性质：不满足自反性和传递性
本体论降格：筛选条件，非本体关系

v4 新增语义：
  "两个资产在递归层级L上等价，当且仅当两者在层级L上都有已完成的走势类型"
  这与 dengjia.md 的 C-1/C-2/C-3 是什么关系？
  是第四个条件（C-4）？还是对 C-2 的级别化扩展？
```
