"""持续图之间的 Wasserstein-1 距离——自实装匹配桥接（issue #324）。

为什么不用 persim.wasserstein
----------------------------
persim 0.3.8 的 ``wasserstein()`` 用 ``sklearn.metrics.pairwise.
pairwise_distances`` 算点对点代价矩阵，而 sklearn 的 euclidean_distances 走
平方展开 ‖x‖² + ‖y‖² − 2x·y（BLAS gemm）。该展开对**完全相同的点对**给出
非零浮点残差（实测 ~1e-7 量级），且残差随平台 BLAS 实现变化：两个恒等
diagram 的 W1 在 ubuntu x86 上得 2.38e-07、在 macOS arm64 上得 0.0——这是
issue #324 里 CI 红 / 本机绿的直接原因。

``scipy.spatial.distance.cdist`` 逐坐标做差再取范数，相同点必给精确 0.0，
且与平台无关。编排者裁定（#324）：生产链改 cdist，不做平方展开——零距离
平台无关精确为零；由此产生的数值微变在噪声级，相关基线照实重锚。

算法
----
本实现与 persim 0.3.8 至少有三处已确认差别：

1. 图↔图代价块：本实现用 ``scipy.spatial.distance.cdist``；persim 用
   ``sklearn.metrics.pairwise.pairwise_distances``（平方展开）。
2. 零距离精确性受两处而非一处影响：除上述代价矩阵外，persim 的对角线垂距
   走旋转矩阵 ``-birth*sin(pi/4) + death*cos(pi/4)``；NumPy 中
   ``sin(pi/4)=0.7071067811865475``、``cos(pi/4)=0.7071067811865476``，
   故 birth=death 时得到约 ``1.1e-16``，本实现的
   ``(death-birth)*sqrt(1/2)`` 得精确 ``0``。
3. 非有限值和额外列：persim ``warnings.warn`` 后丢弃非有限点，并允许额外列
   （忽略之）；本实现对非有限值直接 ``ValueError``，且要求恰为两列。

其余流程仍是构造点↔点、点↔对角线代价块，再由
``scipy.optimize.linear_sum_assignment`` 求最小权完美匹配，代价和即 W1。

有效域
------
仅 p = 1。本模块**不**提供 p ≠ 1 的 Wasserstein-p，也不提供匹配明细
（persim 0.3.8 的签名 ``wasserstein(dgm1, dgm2, matching=False)`` 同样不带
order 参数）。bottleneck 距离（W∞，基于 Hera）仍走 persim，不在本模块内。

输入要求：shape (n, 2) 的 (birth, death) 数组，全部有限。非有限 death 上
W1 无定义，本模块直接报错，不静默丢点。

认识论等级：**L0**（纯算法，不依赖数据）。

概念溯源标签
-----------
- Wasserstein-1 距离（持续图匹配）[TDA:标准定义]
- cdist 代价矩阵裁定 [新缠论:issue #324 编排者裁定]
"""

from __future__ import annotations

import numpy as np
from scipy.optimize import linear_sum_assignment
from scipy.spatial.distance import cdist

__all__ = ["wasserstein_1"]

# cos(π/4) = sin(π/4)：旋转 45° 后点到对角线的垂距 = (death − birth) * 该系数
_SQRT_HALF: float = float(np.sqrt(0.5))


def _diagonal_block(pts: np.ndarray) -> np.ndarray:
    """点↔对角线投影的代价块：块对角放垂距 (death−birth)/√2，其余 inf。

    非对角位置为 inf ⟹ 一个点只能匹配"属于自己的"对角线投影。
    """
    block = np.full((pts.shape[0], pts.shape[0]), np.inf, dtype=float)
    np.fill_diagonal(block, (pts[:, 1] - pts[:, 0]) * _SQRT_HALF)
    return block


def _as_diagram(dgm: np.ndarray, name: str) -> np.ndarray:
    """校验并规范化持续图输入为 shape (n, 2) 的 float 数组。

    Parameters
    ----------
    dgm : np.ndarray
        (n, 2) 的 (birth, death) 数组；空图可为 (0, 2) 或任意 size 0 数组。
    name : str
        参数名（仅用于报错信息）。

    Returns
    -------
    np.ndarray
        shape (n, 2) 的 float 数组，空图规范化为 (0, 2)。

    Raises
    ------
    ValueError
        形状不是 (n, 2)，或含非有限值（W1 在含无穷 bar 的图上无定义）。
    """
    arr = np.asarray(dgm, dtype=float)
    if arr.size == 0:
        return np.empty((0, 2), dtype=float)
    if arr.ndim != 2 or arr.shape[1] != 2:
        raise ValueError(
            f"{name} 必须是 shape (n, 2) 的 (birth, death) 数组，实收 "
            f"{arr.shape}"
        )
    if not np.isfinite(arr).all():
        raise ValueError(
            f"{name} 含非有限 (birth, death)：Wasserstein-1 在含无穷 bar 的"
            "持续图上无定义。调用方须先封顶或丢弃无穷 bar。"
        )
    return arr


def wasserstein_1(dgm_a: np.ndarray, dgm_b: np.ndarray) -> float:
    """两个持续图之间的 Wasserstein-1 距离（最小权完美匹配的代价和）。

    匹配语义见模块 docstring：点↔点用 cdist 直接差分（**不**用 sklearn 的
    平方展开，见 #324——平方展开在零距离处留 ~1e-7 残差且平台相关），
    点↔对角线用垂距。

    Parameters
    ----------
    dgm_a, dgm_b : np.ndarray
        shape (n, 2) 的 (birth, death) 数组，death 须有限。空图用 (0, 2)。

    Returns
    -------
    float
        W1 距离（≥ 0）。两图逐点恒等时**精确**为 0.0（平台无关）。

    Raises
    ------
    ValueError
        输入形状不是 (n, 2) 或含非有限值。
    """
    s = _as_diagram(dgm_a, "dgm_a")
    t = _as_diagram(dgm_b, "dgm_b")

    if s.shape[0] == 0 and t.shape[0] == 0:
        return 0.0
    # 空图退化为对角线单点：其余点只能匹配到对角线上
    if s.shape[0] == 0:
        s = np.zeros((1, 2), dtype=float)
    if t.shape[0] == 0:
        t = np.zeros((1, 2), dtype=float)

    m = s.shape[0]
    n = t.shape[0]

    cost = np.zeros((m + n, m + n), dtype=float)
    # 图↔图：cdist 逐坐标差分，相同点给精确 0.0（#324 裁定）
    cost[:m, :n] = cdist(s, t)

    # 图↔对角线：块对角放垂距，块内其余置 inf（禁止跨点匹配对角线投影）
    cost[:m, n:] = _diagonal_block(s)
    cost[m:, :n] = _diagonal_block(t)

    # 右下块（对角线↔对角线）保持 0

    rows, cols = linear_sum_assignment(cost)
    return float(cost[rows, cols].sum())
